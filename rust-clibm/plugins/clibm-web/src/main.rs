//! CLIBM-WEB — local web dashboard for a CLIBM project.
//!
//! Run it inside a project with `clibm plugin run web` (or `clibm-web --port 7878`).
//! Every read and write goes through `clibm_core`, so the dashboard follows exactly the
//! same rules as the CLI (dependencies, WIP limits, hooks, auto-commit).
//!
//! Security: the server only listens on 127.0.0.1, sends no CORS headers, rejects
//! requests whose `Host`/`Origin` is not this local server (blocks other websites and
//! DNS rebinding), and requires a per-start random token on every write.

use anyhow::{anyhow, bail, Result};
use clibm_core::engine::ClibmEngine;
use clibm_core::models::{TaskPayload, TaskPriority, OBJ_TASK};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::hash_map::RandomState;
use std::env;
use std::hash::{BuildHasher, Hasher};
use tiny_http::{Header, Method, Response, Server};

const PLUGIN_NAME: &str = "clibm-web";
const PLUGIN_DESCRIPTION: &str = "Self-hosted local web dashboard for CLIBM projects (127.0.0.1)";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["http-bridge"];

const DEFAULT_PORT: u16 = 7878;
const EMBEDDED_UI_HTML: &str = include_str!("ui.html");
const TOKEN_PLACEHOLDER: &str = "__CLIBM_TOKEN__";

const USAGE: &str = "clibm-web - local web dashboard for the CLIBM project in the current folder

USAGE:
  clibm-web [--port <PORT>]
  clibm plugin run web -- [--port <PORT>]

OPTIONS:
  -p, --port <PORT>   Port on 127.0.0.1 (default 7878)
      --manifest      Print this plugin's registry entry (used by installers)
  -h, --help          Show this help";

/// Per-server settings used to authorize requests.
#[derive(Debug, Clone)]
pub struct ServerCtx {
    pub port: u16,
    pub token: String,
}

/// An HTTP answer independent of tiny_http (easy to test).
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Reply {
    fn json(status: u16, value: Value) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: value.to_string(),
        }
    }
    fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, json!({ "ok": false, "error": message.into() }))
    }
}

/// A random token for this server run: BLAKE3 over OS-seeded hasher output, the time
/// and the process id.
fn generate_token() -> String {
    let mut seed = Vec::new();
    for _ in 0..4 {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        seed.extend_from_slice(&h.finish().to_le_bytes());
    }
    seed.extend_from_slice(&std::process::id().to_le_bytes());
    blake3::hash(&seed).to_hex()[..32].to_string()
}

fn is_local_host(value: &str, port: u16) -> bool {
    let v = value.trim().to_ascii_lowercase();
    v == format!("127.0.0.1:{}", port) || v == format!("localhost:{}", port)
}

/// Checks that a request comes from this dashboard: the `Host` must be this local
/// server (stops DNS rebinding), an `Origin`, when sent, must be this server too (stops
/// other websites), and writes must carry the session token.
pub fn authorize(
    ctx: &ServerCtx,
    host: Option<&str>,
    origin: Option<&str>,
    token: Option<&str>,
    is_write: bool,
) -> Result<(), Reply> {
    match host {
        Some(h) if is_local_host(h, ctx.port) => {}
        _ => return Err(Reply::error(403, "Forbidden: unexpected Host header")),
    }
    if let Some(o) = origin {
        let o = o.trim().to_ascii_lowercase();
        let allowed = o
            .strip_prefix("http://")
            .map(|rest| is_local_host(rest, ctx.port))
            .unwrap_or(false);
        if !allowed {
            return Err(Reply::error(403, "Forbidden: cross-origin request"));
        }
    }
    if is_write && token != Some(ctx.token.as_str()) {
        return Err(Reply::error(403, "Forbidden: missing or invalid session token"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct MoveRequest {
    task_id: String,
    column_id: String,
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
struct NewTaskRequest {
    title: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    column_id: Option<String>,
}

#[derive(Deserialize)]
struct CommitRequest {
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct WorkspaceRef {
    workspace: String,
}

#[derive(Deserialize)]
struct NewWorkspaceRequest {
    name: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct EditWorkspaceRequest {
    workspace: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct DeleteWorkspaceRequest {
    workspace: String,
    #[serde(default)]
    move_to: Option<String>,
}

#[derive(Deserialize)]
struct ProjectRef {
    project: String,
}

#[derive(Deserialize)]
struct NewProjectRequest {
    name: String,
    code: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    workspace: Option<String>,
    #[serde(default)]
    switch: bool,
}

#[derive(Deserialize)]
struct EditProjectRequest {
    project: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct DeleteProjectRequest {
    project: String,
    #[serde(default)]
    move_to: Option<String>,
}

/// Every workspace with its projects, for the switchers and the Manage dialog.
fn workspaces_snapshot(engine: &ClibmEngine) -> Result<Value> {
    let projects = engine.list_projects(true)?;
    let out: Vec<Value> = engine
        .list_workspaces()?
        .into_iter()
        .map(|w| {
            let prjs: Vec<Value> = projects
                .iter()
                .filter(|p| p.workspace_id == w.id)
                .map(|p| {
                    json!({
                        "id": p.id,
                        "code": p.project.code,
                        "name": p.project.name,
                        "description": p.project.description,
                        "tasks": p.tasks,
                        "active": p.active,
                    })
                })
                .collect();
            json!({
                "id": w.id,
                "name": w.workspace.name,
                "description": w.workspace.description,
                "active": w.active,
                "tasks": w.tasks,
                "projects": prjs,
            })
        })
        .collect();
    Ok(Value::Array(out))
}

/// `{ ok: true, result }` on success, 409 with the core's message on failure.
fn core_reply<T: serde::Serialize>(result: Result<T>) -> Reply {
    match result {
        Ok(r) => Reply::json(200, json!({ "ok": true, "result": r })),
        Err(e) => Reply::error(409, e.to_string()),
    }
}

/// Parses the body as `T` and runs `f` with it through [`core_reply`].
fn with_body<T, R>(body: &str, f: impl FnOnce(T) -> Result<R>) -> Reply
where
    T: serde::de::DeserializeOwned,
    R: serde::Serialize,
{
    match parse_body::<T>(body) {
        Ok(req) => core_reply(f(req)),
        Err(reply) => reply,
    }
}

/// Handles the workspace and project routes; `None` when `path` is not one of them.
fn handle_management(engine: &ClibmEngine, path: &str, body: &str) -> Option<Reply> {
    let reply = match path {
        "/api/workspace/switch" => with_body(body, |r: WorkspaceRef| {
            engine
                .switch_workspace(Some(&r.workspace))
                .map(|(ws, commit)| json!({ "workspace": ws, "commit": commit }))
        }),
        "/api/workspace" => with_body(body, |r: NewWorkspaceRequest| {
            engine
                .create_workspace(&r.name, r.code.as_deref().filter(|c| !c.trim().is_empty()), &r.description)
                .map(|(ws, hash, commit)| json!({ "workspace": ws, "hash": hash, "commit": commit }))
        }),
        "/api/workspace/edit" => with_body(body, |r: EditWorkspaceRequest| {
            engine.edit_workspace(&r.workspace, r.name, r.description)
        }),
        "/api/workspace/delete" => with_body(body, |r: DeleteWorkspaceRequest| {
            engine.delete_workspace(&r.workspace, r.move_to.as_deref().filter(|m| !m.is_empty()))
        }),
        "/api/project/switch" => with_body(body, |r: ProjectRef| {
            engine
                .switch_project(&r.project)
                .map(|(prj, commit)| json!({ "project": prj, "commit": commit }))
        }),
        "/api/project" => with_body(body, |r: NewProjectRequest| {
            engine
                .create_project_in(
                    r.workspace.as_deref().filter(|w| !w.is_empty()),
                    &r.name,
                    &r.code,
                    &r.description,
                    r.switch,
                )
                .map(|(prj, hash, commit)| json!({ "project": prj, "hash": hash, "commit": commit }))
        }),
        "/api/project/edit" => with_body(body, |r: EditProjectRequest| {
            engine.edit_project(&r.project, r.name, r.description)
        }),
        "/api/project/delete" => with_body(body, |r: DeleteProjectRequest| {
            engine.delete_project(&r.project, r.move_to.as_deref().filter(|m| !m.is_empty()))
        }),
        _ => return None,
    };
    Some(reply)
}

/// Current board of the active project with its tasks.
fn state_snapshot(engine: &ClibmEngine) -> Result<Value> {
    let st = engine.status()?;
    let (tree_hash, tree) = engine.load_working_tree()?;
    let project = engine.active_project(&tree)?;
    let board = engine.project_board(&tree, &project)?;
    let mut tasks = Vec::new();
    for (id, hash) in &tree.tasks {
        let task: TaskPayload = engine.cas.get_typed_payload(hash, OBJ_TASK)?;
        if task.project_id == project.id && task.board_id == board.id {
            tasks.push(json!({ "id": id, "hash": hash, "task": task }));
        }
    }
    Ok(json!({
        "ok": true,
        "workspace": { "id": st.active_workspace_id, "name": st.active_workspace_name },
        "workspaces": workspaces_snapshot(engine)?,
        "head_commit": st.head_commit,
        "commits": st.commits,
        "history_note": st.history_note,
        "staged_tree_hash": tree_hash,
        "dirty": st.dirty,
        "auto_commit": st.auto_commit,
        "project": { "id": project.id, "code": project.code, "name": project.name },
        "board": board,
        "tasks": tasks,
    }))
}

fn parse_body<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, Reply> {
    serde_json::from_str(body).map_err(|e| Reply::error(400, format!("Invalid JSON body: {}", e)))
}

/// Routes one request. `host`, `origin` and `token` are the raw header values.
#[allow(clippy::too_many_arguments)]
pub fn handle_request(
    engine: &ClibmEngine,
    ctx: &ServerCtx,
    method: &str,
    url: &str,
    host: Option<&str>,
    origin: Option<&str>,
    token: Option<&str>,
    body: &str,
) -> Reply {
    let path = url.split('?').next().unwrap_or("/");
    let is_write = method != "GET" && method != "HEAD";
    if let Err(reply) = authorize(ctx, host, origin, token, is_write) {
        return reply;
    }

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => Reply {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: EMBEDDED_UI_HTML.replace(TOKEN_PLACEHOLDER, &ctx.token),
        },
        ("GET", "/api/state") => match state_snapshot(engine) {
            Ok(v) => Reply::json(200, v),
            Err(e) => Reply::error(500, e.to_string()),
        },
        ("GET", "/api/fsck") => match engine.cas.fsck() {
            Ok(report) => Reply::json(
                200,
                json!({ "ok": report.corrupted_hashes.is_empty(), "report": report }),
            ),
            Err(e) => Reply::error(500, e.to_string()),
        },
        ("POST", "/api/move") => {
            let req: MoveRequest = match parse_body(body) {
                Ok(r) => r,
                Err(reply) => return reply,
            };
            match engine.move_task(&req.task_id, &req.column_id, req.force) {
                Ok(res) => Reply::json(200, json!({ "ok": true, "result": res })),
                Err(e) => Reply::error(409, e.to_string()),
            }
        }
        ("POST", "/api/task") => {
            let req: NewTaskRequest = match parse_body(body) {
                Ok(r) => r,
                Err(reply) => return reply,
            };
            let default_priority = engine
                .cas
                .load_config()
                .map(|c| c.default_priority)
                .unwrap_or_else(|_| "medium".to_string());
            let priority =
                TaskPriority::from_str_lossy(req.priority.as_deref().unwrap_or(&default_priority));
            let column = req.column_id.unwrap_or_else(|| "todo".to_string());
            match engine.create_task(
                &req.title,
                "",
                priority,
                &column,
                None,
                None,
                vec![],
                vec![],
                false,
            ) {
                Ok((task, hash, commit)) => Reply::json(
                    200,
                    json!({ "ok": true, "id": task.id, "hash": hash, "commit": commit }),
                ),
                Err(e) => Reply::error(409, e.to_string()),
            }
        }
        ("POST", "/api/commit") => {
            let req: CommitRequest = match parse_body(body) {
                Ok(r) => r,
                Err(reply) => return reply,
            };
            let message = if req.message.trim().is_empty() {
                "Snapshot committed from clibm-web".to_string()
            } else {
                req.message
            };
            match engine.commit(&message, None) {
                Ok(Some(commit)) => Reply::json(200, json!({ "ok": true, "commit": commit })),
                Ok(None) => Reply::json(
                    200,
                    json!({ "ok": true, "commit": null, "message": "Nothing to commit: the working tree matches HEAD." }),
                ),
                Err(e) => Reply::error(409, e.to_string()),
            }
        }
        ("POST", p) => handle_management(engine, p, body).unwrap_or_else(|| Reply::error(404, "Not found")),
        _ => Reply::error(404, "Not found"),
    }
}

fn header_value(request: &tiny_http::Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_string())
}

fn parse_port(args: &[String]) -> Result<u16> {
    let mut port = DEFAULT_PORT;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow!("--port needs a value\n\n{}", USAGE))?;
                port = value
                    .parse::<u16>()
                    .map_err(|_| anyhow!("Invalid port '{}'", value))?;
                i += 2;
            }
            other => bail!("Unknown argument '{}'\n\n{}", other, USAGE),
        }
    }
    Ok(port)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--manifest") {
        println!("{}", manifest_json(&args));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", USAGE);
        return Ok(());
    }
    let port = match parse_port(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };

    let cwd = env::current_dir()?;
    let engine = ClibmEngine::open(&cwd);
    if !engine.cas.is_initialized() {
        bail!(
            "No CLIBM project (.clibm/) found in '{}' or its parent folders. Run `clibm init` first.",
            cwd.display()
        );
    }

    let bind_addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&bind_addr)
        .map_err(|e| anyhow!("Failed to bind local HTTP server on {}: {}", bind_addr, e))?;
    let ctx = ServerCtx {
        port,
        token: generate_token(),
    };

    println!("====================================================================");
    println!("CLIBM-WEB v{} - local web dashboard", env!("CARGO_PKG_VERSION"));
    println!("Project : {}", engine.cas.workspace_root().display());
    println!("Open    : http://{}/  (also http://localhost:{}/)", bind_addr, port);
    println!("Stop    : Ctrl+C");
    println!("====================================================================");

    for mut request in server.incoming_requests() {
        let method = match request.method() {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            _ => "OTHER",
        };
        let url = request.url().to_string();
        let host = header_value(&request, "Host");
        let origin = header_value(&request, "Origin");
        let token = header_value(&request, "X-Clibm-Token");
        let mut body = String::new();
        if method == "POST" {
            let _ = request.as_reader().read_to_string(&mut body);
        }

        let reply = handle_request(
            &engine,
            &ctx,
            method,
            &url,
            host.as_deref(),
            origin.as_deref(),
            token.as_deref(),
            &body,
        );
        let response = Response::from_string(reply.body)
            .with_status_code(reply.status)
            .with_header(
                Header::from_bytes(&b"Content-Type"[..], reply.content_type.as_bytes()).unwrap(),
            )
            .with_header(Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap())
            .with_header(
                Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..]).unwrap(),
            );
        let _ = request.respond(response);
    }

    Ok(())
}

/// `--manifest [--executable <path>] [--hooks a,b]`: prints this plugin's registry entry
/// (one JSON line). Installers and `clibm plugin install` store it in
/// `~/.clibm/plugins/registry.json`.
fn manifest_json(args: &[String]) -> String {
    let value_after = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let hooks: Vec<String> = match value_after("--hooks") {
        Some(csv) => csv
            .split(',')
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
            .collect(),
        None => DEFAULT_HOOKS.iter().map(|h| h.to_string()).collect(),
    };
    serde_json::json!({
        "name": PLUGIN_NAME,
        "version": env!("CARGO_PKG_VERSION"),
        "description": PLUGIN_DESCRIPTION,
        "enabled": true,
        "hooks": hooks,
        "executable": value_after("--executable"),
        "tools": serde_json::json!([]),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ServerCtx {
        ServerCtx {
            port: 7878,
            token: "secret-token".to_string(),
        }
    }

    #[test]
    fn authorize_checks_host_origin_and_token() {
        let c = ctx();
        // Same-origin page load and API reads.
        assert!(authorize(&c, Some("127.0.0.1:7878"), None, None, false).is_ok());
        assert!(authorize(&c, Some("localhost:7878"), Some("http://localhost:7878"), None, false).is_ok());
        // DNS rebinding: a foreign host name pointing at 127.0.0.1.
        assert_eq!(authorize(&c, Some("evil.example:7878"), None, None, false).unwrap_err().status, 403);
        assert_eq!(authorize(&c, None, None, None, false).unwrap_err().status, 403);
        // Another website posting to the local server.
        assert_eq!(
            authorize(&c, Some("127.0.0.1:7878"), Some("http://evil.example"), Some("secret-token"), true)
                .unwrap_err()
                .status,
            403
        );
        // Writes need the token.
        assert_eq!(authorize(&c, Some("127.0.0.1:7878"), None, None, true).unwrap_err().status, 403);
        assert_eq!(authorize(&c, Some("127.0.0.1:7878"), None, Some("wrong"), true).unwrap_err().status, 403);
        assert!(authorize(&c, Some("127.0.0.1:7878"), Some("http://127.0.0.1:7878"), Some("secret-token"), true).is_ok());
    }

    #[test]
    fn tokens_are_random_and_injected_into_the_page() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        let dir = std::env::temp_dir().join(format!("clibm-web-page-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = ClibmEngine::open_exact(&dir).without_hooks();
        let page = handle_request(&engine, &ctx(), "GET", "/", Some("127.0.0.1:7878"), None, None, "");
        assert_eq!(page.status, 200);
        assert!(page.body.contains("secret-token"));
        assert!(!page.body.contains(TOKEN_PLACEHOLDER));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn web_writes_produce_valid_core_objects() {
        use clibm_core::versioning::VersioningEngine;

        let dir = std::env::temp_dir().join(format!("clibm-web-writes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = ClibmEngine::open_exact(&dir).without_hooks();
        engine.init("ws", "APP", "App").unwrap();
        let c = ctx();
        let host = Some("127.0.0.1:7878");
        let post = |url: &str, body: &str| {
            handle_request(&engine, &c, "POST", url, host, Some("http://127.0.0.1:7878"), Some("secret-token"), body)
        };

        let created = post("/api/task", r#"{"title":"From the web","priority":"high"}"#);
        assert_eq!(created.status, 200, "{}", created.body);
        let moved = post("/api/move", r#"{"task_id":"101","column_id":"in-progress"}"#);
        assert_eq!(moved.status, 200, "{}", moved.body);
        // Core rules apply: unknown column -> 409 with the core's message.
        let bad = post("/api/move", r#"{"task_id":"APP-101","column_id":"nowhere"}"#);
        assert_eq!(bad.status, 409);
        assert!(bad.body.contains("does not exist"), "{}", bad.body);
        // Auto-commit is on, so a manual commit has nothing to do.
        let commit = post("/api/commit", r#"{"message":"web"}"#);
        assert!(commit.body.contains("Nothing to commit"), "{}", commit.body);

        // Everything the web wrote is a valid core object and the history is readable.
        let report = engine.cas.fsck().unwrap();
        assert_eq!(report.verified_objects, report.total_objects);
        assert!(report.corrupted_hashes.is_empty() && report.legacy_hashes.is_empty());
        let log = VersioningEngine::log_history(&engine.cas, None).unwrap();
        assert!(log.len() >= 3, "expected init + add + move commits, got {}", log.len());

        let state = handle_request(&engine, &c, "GET", "/api/state", host, None, None, "");
        let v: Value = serde_json::from_str(&state.body).unwrap();
        assert_eq!(v["tasks"][0]["task"]["column_id"], "in-progress");
        assert_eq!(v["project"]["code"], "APP");
        let _ = std::fs::remove_dir_all(&dir);
    }

    const MANAGEMENT_ROUTES: &[&str] = &[
        "/api/workspace/switch",
        "/api/workspace",
        "/api/workspace/edit",
        "/api/workspace/delete",
        "/api/project/switch",
        "/api/project",
        "/api/project/edit",
        "/api/project/delete",
    ];

    #[test]
    fn management_routes_need_the_token_and_same_origin() {
        let dir = std::env::temp_dir().join(format!("clibm-web-auth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = ClibmEngine::open_exact(&dir).without_hooks();
        engine.init("ws", "APP", "App").unwrap();
        let host = Some("127.0.0.1:7878");
        for route in MANAGEMENT_ROUTES {
            let body = r#"{"name":"x","code":"X","workspace":"WS-01","project":"APP"}"#;
            let no_token = handle_request(&engine, &ctx(), "POST", route, host, None, None, body);
            assert_eq!(no_token.status, 403, "{} without token", route);
            let foreign = handle_request(
                &engine, &ctx(), "POST", route, host, Some("http://evil.example"), Some("secret-token"), body,
            );
            assert_eq!(foreign.status, 403, "{} from another origin", route);
        }
        // Nothing was changed.
        assert_eq!(engine.list_workspaces().unwrap().len(), 1);
        assert_eq!(engine.list_projects(true).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspaces_and_projects_can_be_managed_from_the_web() {
        use clibm_core::versioning::VersioningEngine;

        let dir = std::env::temp_dir().join(format!("clibm-web-manage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = ClibmEngine::open_exact(&dir).without_hooks();
        engine.init("ws", "APP", "App").unwrap();
        let c = ctx();
        let host = Some("127.0.0.1:7878");
        let post = |url: &str, body: &str| {
            handle_request(&engine, &c, "POST", url, host, Some("http://127.0.0.1:7878"), Some("secret-token"), body)
        };
        let state = || -> Value {
            let r = handle_request(&engine, &c, "GET", "/api/state", host, None, None, "");
            assert_eq!(r.status, 200, "{}", r.body);
            serde_json::from_str(&r.body).unwrap()
        };
        let commits = || VersioningEngine::log_history(&engine.cas, None).unwrap().len();
        let ok = |r: Reply| -> Value {
            assert_eq!(r.status, 200, "{}", r.body);
            serde_json::from_str(&r.body).unwrap()
        };

        assert_eq!(post("/api/task", r#"{"title":"App task"}"#).status, 200);

        // Create a workspace and a project in it without switching.
        let before = commits();
        ok(post("/api/workspace", r#"{"name":"backend","code":"BE","description":"Server side"}"#));
        ok(post("/api/project", r#"{"name":"Jobs","code":"job","workspace":"backend"}"#));
        assert_eq!(commits(), before + 2, "one commit per change");
        let s = state();
        assert_eq!(s["workspace"]["id"], "WS-01");
        assert_eq!(s["project"]["code"], "APP");
        let ws = s["workspaces"].as_array().unwrap();
        assert_eq!(ws.len(), 2);
        let be = ws.iter().find(|w| w["name"] == "backend").unwrap();
        assert_eq!(be["description"], "Server side");
        assert_eq!(be["projects"].as_array().unwrap().len(), 2);

        // Switching to a project of another workspace switches the workspace too.
        ok(post("/api/project/switch", r#"{"project":"JOB"}"#));
        let s = state();
        assert_eq!(s["workspace"]["name"], "backend");
        assert_eq!(s["project"]["code"], "JOB");
        ok(post("/api/workspace/switch", r#"{"workspace":"ws"}"#));
        assert_eq!(state()["project"]["code"], "APP");

        // Edit both.
        let res = ok(post("/api/workspace/edit", r#"{"workspace":"backend","name":"Server"}"#));
        assert_eq!(res["result"]["changed"], true);
        ok(post("/api/project/edit", r#"{"project":"JOB","description":"Cron jobs"}"#));
        let s = state();
        let server = s["workspaces"].as_array().unwrap().iter().find(|w| w["id"] == "WS-02").unwrap().clone();
        assert_eq!(server["name"], "Server");
        let job = server["projects"].as_array().unwrap().iter().find(|p| p["code"] == "JOB").unwrap().clone();
        assert_eq!(job["description"], "Cron jobs");

        // Refusals come back as 409 with the core's message; bad JSON is a 400.
        let dup = post("/api/workspace", r#"{"name":"server"}"#);
        assert_eq!(dup.status, 409);
        assert!(dup.body.contains("already used"), "{}", dup.body);
        let busy = post("/api/project/delete", r#"{"project":"APP"}"#);
        assert_eq!(busy.status, 409, "{}", busy.body);
        assert!(busy.body.contains("only project"), "{}", busy.body);
        assert_eq!(post("/api/project", "{not json").status, 400);
        assert_eq!(post("/api/nowhere", "{}").status, 404);

        // Delete a project with tasks by moving them, then a workspace by moving its projects.
        ok(post("/api/project", r#"{"name":"Extra","code":"EXT","switch":true}"#));
        assert_eq!(state()["project"]["code"], "EXT");
        let refused = post("/api/project/delete", r#"{"project":"APP"}"#);
        assert!(refused.body.contains("still has 1 task"), "{}", refused.body);
        let res = ok(post("/api/project/delete", r#"{"project":"APP","move_to":"EXT"}"#));
        assert_eq!(res["result"]["moved_tasks"], json!(["APP-101"]));
        let refused = post("/api/workspace/delete", r#"{"workspace":"ws"}"#);
        assert!(refused.body.contains("still has 1 task"), "{}", refused.body);
        let res = ok(post("/api/workspace/delete", r#"{"workspace":"ws","move_to":"Server"}"#));
        assert_eq!(res["result"]["active_workspace_id"], "WS-02");
        let s = state();
        assert_eq!(s["workspaces"].as_array().unwrap().len(), 1);
        assert_eq!(s["project"]["code"], "EXT", "the active project moved along");
        assert_eq!(s["tasks"].as_array().unwrap().len(), 1);

        let report = engine.cas.fsck().unwrap();
        assert_eq!(report.verified_objects, report.total_objects);
        assert!(report.corrupted_hashes.is_empty() && report.legacy_hashes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn port_argument_parsing() {
        assert_eq!(parse_port(&[]).unwrap(), DEFAULT_PORT);
        assert_eq!(parse_port(&["--port".into(), "9000".into()]).unwrap(), 9000);
        assert!(parse_port(&["--port".into()]).is_err());
        assert!(parse_port(&["--bogus".into()]).is_err());
    }
}
