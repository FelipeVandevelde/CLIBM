//! CLIBM-WEB — Standalone Single-Binary Self-Hosted Web Dashboard Plugin
//!
//! Built and installed from within its own plugin folder (`plugins/clibm-web`).
//! Once installed (`./install.sh`, `.\install.ps1`, or `clibm plugin install .`),
//! run it inside any ongoing CLIBM project via:
//!   - `clibm plugin run web` (or `clibm plugin run clibm-web -- --port 7878`)
//!   - `clibm-web --port 7878`

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};
#[cfg(feature = "mcp")]
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server};

const FORMAT_VERSION: &str = "0.1.0";
const OBJ_PROJECT: &str = "project";
const OBJ_BOARD: &str = "board";
const OBJ_TASK: &str = "task";
const OBJ_TREE: &str = "tree";
const OBJ_COMMIT: &str = "commit";

const EMBEDDED_UI_HTML: &str = include_str!("ui.html");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CasObject {
    #[serde(rename = "Type")]
    object_type: String,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "Hash")]
    hash: String,
    #[serde(rename = "Payload")]
    payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectPayload {
    id: String,
    workspace_id: String,
    code: String,
    name: String,
    #[serde(default)]
    board_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KanbanColumn {
    id: String,
    name: String,
    order: usize,
    wip_limit: Option<usize>,
    #[serde(default)]
    is_completed_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KanbanBoardPayload {
    id: String,
    project_id: String,
    name: String,
    columns: Vec<KanbanColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskComment {
    id: String,
    author: String,
    timestamp: String,
    body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskPayload {
    id: String,
    project_id: String,
    board_id: String,
    column_id: String,
    title: String,
    description: String,
    priority: String,
    assignee: Option<String>,
    due_date: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    comments: Vec<TaskComment>,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TreeSnapshotPayload {
    active_workspace_id: String,
    active_project_id: String,
    workspaces: BTreeMap<String, String>,
    projects: BTreeMap<String, String>,
    boards: BTreeMap<String, String>,
    tasks: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitPayload {
    tree_hash: String,
    parents: Vec<String>,
    author: String,
    timestamp: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
    description: String,
    enabled: bool,
    hooks: Vec<String>,
    executable: Option<String>,
}

#[derive(Deserialize)]
struct MoveRequest {
    task_id: String,
    column_id: String,
}

#[derive(Deserialize)]
struct NewTaskRequest {
    title: String,
    priority: String,
    column_id: String,
}

#[derive(Deserialize)]
struct CommitRequest {
    message: String,
}

fn normalize_value(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut sorted: BTreeMap<String, Value> = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), normalize_value(v));
            }
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_value).collect()),
        other => other.clone(),
    }
}

fn compute_object_hash<T: Serialize>(object_type: &str, payload: &T) -> Result<(String, Value)> {
    let raw = serde_json::to_value(payload)?;
    let normalized = normalize_value(&raw);
    let canonical_payload = serde_json::to_string(&normalized)?;
    let preimage = format!(
        "{}:{}:{}",
        object_type, FORMAT_VERSION, canonical_payload
    );
    let hash = blake3::hash(preimage.as_bytes()).to_hex().to_string();
    Ok((hash, normalized))
}

struct ProjectCas {
    project_root: PathBuf,
    clibm_dir: PathBuf,
}

impl ProjectCas {
    fn discover_ongoing(cwd: &Path) -> Result<Self> {
        if let Ok(env_root) = env::var("CLIBM_PROJECT_ROOT") {
            let p = PathBuf::from(env_root);
            let candidate = p.join(".clibm");
            if candidate.exists() && candidate.join("objects").exists() {
                return Ok(Self {
                    project_root: p,
                    clibm_dir: candidate,
                });
            }
        }

        for ancestor in cwd.ancestors() {
            let candidate = ancestor.join(".clibm");
            if candidate.exists() && candidate.join("objects").exists() {
                return Ok(Self {
                    project_root: ancestor.to_path_buf(),
                    clibm_dir: candidate,
                });
            }
        }

        bail!(
            "No ongoing CLIBM project (.clibm/) found in '{}' or its parent directories.\n\
             Navigate to an initialized CLIBM project folder (`cd <your-project>` where `clibm init` was run) \
             and execute `clibm plugin run web` or `clibm-web`.",
            cwd.display()
        )
    }

    fn object_path(&self, hash: &str) -> Result<PathBuf> {
        if hash.len() < 6 {
            bail!("Invalid BLAKE3 hash: {}", hash);
        }
        let (shard, rest) = hash.split_at(2);
        Ok(self
            .clibm_dir
            .join("objects")
            .join(shard)
            .join(format!("{}.json", rest)))
    }

    fn get_typed<T: DeserializeOwned>(&self, hash: &str, expected_type: &str) -> Result<T> {
        let path = self.object_path(hash)?;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read CAS object {}", hash))?;
        let obj: CasObject = serde_json::from_str(&content)?;
        if obj.object_type != expected_type {
            bail!(
                "CAS type mismatch for {}: expected {}, got {}",
                hash,
                expected_type,
                obj.object_type
            );
        }
        Ok(serde_json::from_value(obj.payload)?)
    }

    fn put_object<T: Serialize>(&self, object_type: &str, payload: &T) -> Result<String> {
        let (hash, normalized) = compute_object_hash(object_type, payload)?;
        let path = self.object_path(&hash)?;
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let env = CasObject {
                object_type: object_type.to_string(),
                version: FORMAT_VERSION.to_string(),
                hash: hash.clone(),
                payload: normalized,
            };
            fs::write(&path, serde_json::to_string_pretty(&env)?)?;
        }
        Ok(hash)
    }

    fn current_branch(&self) -> Option<String> {
        let head = fs::read_to_string(self.clibm_dir.join("HEAD")).ok()?;
        head.trim()
            .strip_prefix("ref: refs/heads/")
            .map(|s| s.to_string())
    }

    fn resolve_head_commit(&self) -> Option<String> {
        let head = fs::read_to_string(self.clibm_dir.join("HEAD")).ok()?;
        let trimmed = head.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            let bfile = self.clibm_dir.join("refs").join("heads").join(branch);
            fs::read_to_string(bfile)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else if !trimmed.is_empty() {
            Some(trimmed.to_string())
        } else {
            None
        }
    }

    fn load_working_tree(&self) -> Result<(String, TreeSnapshotPayload)> {
        let stage_file = self.clibm_dir.join("STAGE_TREE");
        if stage_file.exists() {
            let h = fs::read_to_string(&stage_file)?.trim().to_string();
            if !h.is_empty() {
                let tree = self.get_typed::<TreeSnapshotPayload>(&h, OBJ_TREE)?;
                return Ok((h, tree));
            }
        }
        if let Some(commit_hash) = self.resolve_head_commit() {
            let commit = self.get_typed::<CommitPayload>(&commit_hash, OBJ_COMMIT)?;
            let tree = self.get_typed::<TreeSnapshotPayload>(&commit.tree_hash, OBJ_TREE)?;
            return Ok((commit.tree_hash, tree));
        }
        bail!("Ongoing project has no staged tree or HEAD commit.")
    }

    fn save_working_tree(&self, tree: &TreeSnapshotPayload) -> Result<String> {
        let hash = self.put_object(OBJ_TREE, tree)?;
        fs::write(self.clibm_dir.join("STAGE_TREE"), format!("{}\n", hash))?;
        Ok(hash)
    }

    fn log_commits(&self) -> Result<Vec<(String, CommitPayload)>> {
        let mut results = Vec::new();
        let mut queue = Vec::new();
        let mut visited = std::collections::HashSet::new();
        if let Some(head) = self.resolve_head_commit() {
            queue.push(head);
        }
        while let Some(curr) = queue.pop() {
            if !visited.insert(curr.clone()) {
                continue;
            }
            if let Ok(commit) = self.get_typed::<CommitPayload>(&curr, OBJ_COMMIT) {
                for p in &commit.parents {
                    queue.push(p.clone());
                }
                results.push((curr, commit));
            }
        }
        results.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
        Ok(results)
    }

    fn fsck(&self) -> Result<Value> {
        let mut total = 0usize;
        let mut verified = 0usize;
        let mut corrupted = Vec::new();
        let obj_dir = self.clibm_dir.join("objects");
        if obj_dir.exists() {
            for shard in fs::read_dir(obj_dir)? {
                let shard = shard?;
                if shard.path().is_dir() {
                    for entry in fs::read_dir(shard.path())? {
                        let entry = entry?;
                        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                            total += 1;
                            let text = fs::read_to_string(entry.path())?;
                            if let Ok(obj) = serde_json::from_str::<CasObject>(&text) {
                                let (recomputed, _) =
                                    compute_object_hash(&obj.object_type, &obj.payload)?;
                                if recomputed == obj.hash {
                                    verified += 1;
                                } else {
                                    corrupted.push(obj.hash);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(json!({
            "total_objects": total,
            "verified_objects": verified,
            "corrupted_hashes": corrupted,
        }))
    }
}

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap()
}

fn html_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()
}

fn cors_headers() -> Vec<Header> {
    vec![
        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap(),
        Header::from_bytes(
            &b"Access-Control-Allow-Headers"[..],
            &b"Content-Type, X-MCP-Transport, Authorization"[..],
        )
        .unwrap(),
    ]
}

fn respond_json(request: tiny_http::Request, body: String, status: u16) {
    let mut resp = Response::from_string(body)
        .with_status_code(status)
        .with_header(json_header());
    for h in cors_headers() {
        resp.add_header(h);
    }
    let _ = request.respond(resp);
}

fn build_state_snapshot(cas: &ProjectCas) -> Result<Value> {
    let (tree_hash, tree) = cas.load_working_tree()?;
    let branch = cas.current_branch().unwrap_or_else(|| "detached".to_string());
    let board_hash = tree
        .boards
        .values()
        .next()
        .ok_or_else(|| anyhow!("No board"))?;
    let board: KanbanBoardPayload = cas.get_typed(board_hash, OBJ_BOARD)?;

    let mut tasks = Vec::new();
    for (tid, thash) in &tree.tasks {
        let t: TaskPayload = cas.get_typed(thash, OBJ_TASK)?;
        tasks.push(json!({
            "id": tid,
            "hash": thash,
            "task": t,
        }));
    }

    let commits = cas.log_commits()?;
    let fsck = cas.fsck()?;

    Ok(json!({
        "branch": branch,
        "staged_tree_hash": tree_hash,
        "board": board,
        "tasks": tasks,
        "commits": commits,
        "fsck": fsck,
    }))
}

fn update_registry_file(reg_file: &Path, manifest: &PluginManifest) -> Result<()> {
    if let Some(parent) = reg_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut list: Vec<PluginManifest> = if reg_file.exists() {
        fs::read_to_string(reg_file)
            .ok()
            .and_then(|s| {
                let clean = s.strip_prefix('\u{FEFF}').unwrap_or(&s);
                serde_json::from_str(clean).ok()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if let Some(existing) = list.iter_mut().find(|p| p.name == manifest.name) {
        *existing = manifest.clone();
    } else {
        list.push(manifest.clone());
    }

    let json = format!("\u{FEFF}{}", serde_json::to_string_pretty(&list)?);
    fs::write(reg_file, json)?;
    Ok(())
}

fn register_plugin_globally(bin_path: Option<String>) -> Result<()> {
    let home = env::var("CLIBM_HOME")
        .or_else(|_| env::var("HOME"))
        .or_else(|_| env::var("USERPROFILE"))
        .context("Could not determine user home directory")?;
    let plugins_dir = PathBuf::from(home).join(".clibm").join("plugins");
    fs::create_dir_all(&plugins_dir)?;
    let reg_file = plugins_dir.join("registry.json");

    let exe = bin_path.or_else(|| {
        env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });

    let manifest = PluginManifest {
        name: "clibm-web".to_string(),
        version: "0.1.0".to_string(),
        description:
            "Single-binary self-hosted local HTTP server & web workbench (127.0.0.1:7878)"
                .to_string(),
        enabled: true,
        hooks: vec!["http-bridge".to_string(), "on-state-query".to_string()],
        executable: exe.clone(),
    };

    update_registry_file(&reg_file, &manifest)?;
    if let Ok(cwd) = env::current_dir() {
        if let Ok(cas) = ProjectCas::discover_ongoing(&cwd) {
            let _ = update_registry_file(
                &cas.clibm_dir.join("plugins").join("registry.json"),
                &manifest,
            );
        }
    }
    println!(
        "Registered plugin 'clibm-web' in global CLIBM registry ({})",
        reg_file.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--contract") {
        println!(
            "{{\"contract\":\"CLIBM_PLUGIN_CONTRACT_V1\",\"name\":\"clibm-web\",\"version\":\"0.1.0\"}}"
        );
        return Ok(());
    }

    if let Some(pos) = args.iter().position(|a| a == "--register") {
        let custom_bin = args.get(pos + 1).cloned();
        return register_plugin_globally(custom_bin);
    }

    let port = args
        .iter()
        .position(|a| a == "--port" || a == "-p")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("7878");

    let disable_mcp = args.iter().any(|a| a == "--no-mcp");

    let bind_addr = format!("127.0.0.1:{}", port);
    let cwd = env::current_dir()?;
    let cas = ProjectCas::discover_ongoing(&cwd)?;

    #[cfg(feature = "mcp")]
    let mcp_adapter = if !disable_mcp {
        let mut mcp_cfg = clibm_mcp::McpConfig::default();
        mcp_cfg.project_root = cas.project_root.clone();
        Some(Arc::new(clibm_mcp::McpAdapter::new(mcp_cfg)))
    } else {
        None
    };
    #[cfg(not(feature = "mcp"))]
    let _ = disable_mcp;

    let server = Server::http(&bind_addr)
        .map_err(|e| anyhow!("Failed to bind local HTTP server on {}: {}", bind_addr, e))?;

    println!("====================================================================");
    println!("CLIBM-WEB v0.1.0 — Unified Local Web Dashboard & AI MCP Server");
    println!("Ongoing Project: {}", cas.project_root.display());
    println!("CAS Repository:  {}", cas.clibm_dir.display());
    println!("Listening:       http://{}", bind_addr);
    println!("  • Web Dashboard : http://{}/", bind_addr);
    #[cfg(feature = "mcp")]
    if mcp_adapter.is_some() {
        println!("  • AI Agent MCP  : http://{}/mcp (JSON-RPC 2.0 & Capabilities)", bind_addr);
    }
    println!("====================================================================");

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();

        // Support CORS preflight
        if method == Method::Options {
            let mut resp = Response::empty(200);
            for h in cors_headers() {
                resp.add_header(h);
            }
            let _ = request.respond(resp);
            continue;
        }

        // Handle MCP endpoint (/mcp or /sse)
        #[cfg(feature = "mcp")]
        if let Some(ref adapter) = mcp_adapter {
            if url == "/mcp" || url == "/sse" {
                if method == Method::Get {
                    let caps = serde_json::to_string(&adapter.capabilities())?;
                    respond_json(request, caps, 200);
                    continue;
                } else if method == Method::Post {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    let rpc_resp = match serde_json::from_str::<clibm_mcp::JsonRpcRequest>(&body) {
                        Ok(req) => adapter.handle_request(req),
                        Err(e) => clibm_mcp::JsonRpcResponse::error(
                            Value::Null,
                            clibm_mcp::JsonRpcError::parse_error(format!("Malformed JSON: {}", e)),
                        ),
                    };
                    let resp_json = serde_json::to_string(&rpc_resp)?;
                    respond_json(request, resp_json, 200);
                    continue;
                }
            }
        }

        // Handle root / and /index.html
        if url == "/" || url == "/index.html" {
            if method == Method::Get {
                #[cfg(feature = "mcp")]
                let prefers_json = request.headers().iter().any(|h| {
                    h.field.equiv("accept")
                        && h.value.as_str().contains("application/json")
                        && !h.value.as_str().contains("text/html")
                });
                #[cfg(not(feature = "mcp"))]
                let prefers_json = false;

                #[cfg(feature = "mcp")]
                if prefers_json {
                    if let Some(ref adapter) = mcp_adapter {
                        let caps = serde_json::to_string(&adapter.capabilities())?;
                        respond_json(request, caps, 200);
                        continue;
                    }
                }

                let _ = request.respond(Response::from_string(EMBEDDED_UI_HTML).with_header(html_header()));
                continue;
            }

            // Support direct JSON-RPC POST to root /
            #[cfg(feature = "mcp")]
            if method == Method::Post {
                if let Some(ref adapter) = mcp_adapter {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    if let Ok(req) = serde_json::from_str::<clibm_mcp::JsonRpcRequest>(&body) {
                        let rpc_resp = adapter.handle_request(req);
                        let resp_json = serde_json::to_string(&rpc_resp)?;
                        respond_json(request, resp_json, 200);
                        continue;
                    }
                }
            }
        }

        if method == Method::Get && url == "/api/state" {
            match build_state_snapshot(&cas) {
                Ok(val) => {
                    respond_json(request, serde_json::to_string(&val)?, 200);
                }
                Err(e) => {
                    respond_json(request, json!({ "error": e.to_string() }).to_string(), 400);
                }
            }
            continue;
        }

        if method == Method::Post && url == "/api/move" {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            if let Ok(req) = serde_json::from_str::<MoveRequest>(&body) {
                if let Ok((_, mut tree)) = cas.load_working_tree() {
                    if let Some(thash) = tree.tasks.get(&req.task_id).cloned() {
                        if let Ok(mut task) = cas.get_typed::<TaskPayload>(&thash, OBJ_TASK) {
                            task.column_id = req.column_id;
                            task.updated_at = Utc::now().to_rfc3339();
                            if let Ok(new_hash) = cas.put_object(OBJ_TASK, &task) {
                                tree.tasks.insert(req.task_id, new_hash.clone());
                                let _ = cas.save_working_tree(&tree);
                                respond_json(
                                    request,
                                    json!({ "ok": true, "hash": new_hash }).to_string(),
                                    200,
                                );
                                continue;
                            }
                        }
                    }
                }
            }
            respond_json(
                request,
                json!({ "ok": false, "error": "Move failed" }).to_string(),
                400,
            );
            continue;
        }

        if method == Method::Post && url == "/api/task" {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            if let Ok(req) = serde_json::from_str::<NewTaskRequest>(&body) {
                if let Ok((_, mut tree)) = cas.load_working_tree() {
                    let prj_id = tree.active_project_id.clone();
                    let prj_code = tree
                        .projects
                        .get(&prj_id)
                        .and_then(|h| cas.get_typed::<ProjectPayload>(h, OBJ_PROJECT).ok())
                        .map(|p| p.code)
                        .unwrap_or_else(|| "CLIBM".to_string());
                    let board_id = tree
                        .boards
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| "BRD-01".to_string());
                    let task_id = format!("{}-{}", prj_code, tree.tasks.len() + 101);
                    let now = Utc::now().to_rfc3339();
                    let new_task = TaskPayload {
                        id: task_id.clone(),
                        project_id: prj_id,
                        board_id,
                        column_id: req.column_id,
                        title: req.title,
                        description: "Created via self-hosted clibm-web".to_string(),
                        priority: req.priority.to_lowercase(),
                        assignee: None,
                        due_date: None,
                        created_at: now.clone(),
                        updated_at: now,
                        tags: vec!["web".to_string()],
                        depends_on: Vec::new(),
                        comments: Vec::new(),
                        metadata: BTreeMap::new(),
                    };
                    if let Ok(thash) = cas.put_object(OBJ_TASK, &new_task) {
                        tree.tasks.insert(task_id, thash);
                        let _ = cas.save_working_tree(&tree);
                    }
                }
            }
            let snap = build_state_snapshot(&cas)?;
            respond_json(request, serde_json::to_string(&snap)?, 200);
            continue;
        }

        if method == Method::Post && url == "/api/commit" {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            if let Ok(req) = serde_json::from_str::<CommitRequest>(&body) {
                if let Ok((tree_hash, _)) = cas.load_working_tree() {
                    let parents = cas.resolve_head_commit().into_iter().collect();
                    let payload = CommitPayload {
                        tree_hash,
                        parents,
                        author: "Web Operator".to_string(),
                        timestamp: Utc::now().to_rfc3339(),
                        message: req.message,
                    };
                    if let Ok(commit_hash) = cas.put_object(OBJ_COMMIT, &payload) {
                        if let Some(branch) = cas.current_branch() {
                            let bfile = cas.clibm_dir.join("refs").join("heads").join(branch);
                            let _ = fs::write(bfile, format!("{}\n", commit_hash));
                        } else {
                            let _ =
                                fs::write(cas.clibm_dir.join("HEAD"), format!("{}\n", commit_hash));
                        }
                    }
                }
            }
            let snap = build_state_snapshot(&cas)?;
            respond_json(request, serde_json::to_string(&snap)?, 200);
            continue;
        }

        let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "mcp")]
    fn test_unified_web_mcp_integration() {
        let mut cfg = clibm_mcp::McpConfig::default();
        cfg.read_only = false;
        let adapter = clibm_mcp::McpAdapter::new(cfg);

        let caps = adapter.capabilities();
        assert!(caps["capabilities"]["tools"].is_object());

        let req = clibm_mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: json!({}),
        };
        let resp = adapter.handle_request(req);
        assert_eq!(resp.id, json!(1));
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_cors_headers_structure() {
        let headers = cors_headers();
        assert_eq!(headers.len(), 3);
    }
}
