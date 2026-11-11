//! CLIBM-TUI — Standalone Ratatui + Crossterm Terminal User Interface Plugin
//!
//! Built and installed from within its own plugin folder (`plugins/clibm-tui`).
//! Once installed (`./install.sh`, `.\install.ps1`, or `clibm plugin install .`),
//! run it inside any ongoing CLIBM project via:
//!   - `clibm plugin run tui` (or `clibm plugin run clibm-tui`)
//!   - `clibm-tui`

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

const FORMAT_VERSION: &str = "0.1.0";
const OBJ_BOARD: &str = "board";
const OBJ_TASK: &str = "task";
const OBJ_TREE: &str = "tree";
const OBJ_COMMIT: &str = "commit";

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
struct KanbanColumn {
    id: String,
    name: String,
    order: usize,
    wip_limit: Option<usize>,
    #[serde(default)]
    is_completed_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WorkflowConfig {
    #[serde(default = "default_true")]
    enforce_dependencies: bool,
    #[serde(default = "default_true")]
    enforce_wip_limits: bool,
    #[serde(default)]
    require_assignee_in_progress: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KanbanBoardPayload {
    id: String,
    project_id: String,
    name: String,
    columns: Vec<KanbanColumn>,
    #[serde(default)]
    workflow: WorkflowConfig,
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
    /// Discovers an ongoing `.clibm` project in `CLIBM_PROJECT_ROOT`, `cwd`, or parent directories.
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
             and execute `clibm plugin run tui` or `clibm-tui`.",
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

    fn fsck(&self) -> Result<(usize, usize)> {
        let mut total = 0usize;
        let mut verified = 0usize;
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
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok((verified, total))
    }
}

struct TuiApp {
    cas: ProjectCas,
    selected_col: usize,
    selected_task_idx: usize,
    status_banner: String,
}

impl TuiApp {
    fn new(cas: ProjectCas) -> Self {
        Self {
            cas,
            selected_col: 0,
            selected_task_idx: 0,
            status_banner: "Ready — [h/l] Column  [j/k] Task  [m] Move Right  [c] Commit  [f] BLAKE3 fsck  [q] Quit".to_string(),
        }
    }

    fn load_state(
        &self,
    ) -> Result<(
        String,
        TreeSnapshotPayload,
        KanbanBoardPayload,
        Vec<(String, TaskPayload)>,
    )> {
        let (tree_hash, tree) = self.cas.load_working_tree()?;
        let board_hash = tree
            .boards
            .values()
            .next()
            .ok_or_else(|| anyhow!("No board found in ongoing project"))?;
        let board: KanbanBoardPayload = self.cas.get_typed(board_hash, OBJ_BOARD)?;

        let mut tasks = Vec::new();
        for (tid, thash) in &tree.tasks {
            let t: TaskPayload = self.cas.get_typed(thash, OBJ_TASK)?;
            tasks.push((tid.clone(), t));
        }
        Ok((tree_hash, tree, board, tasks))
    }

    fn move_selected_task_forward(&mut self) {
        if let Ok((_, mut tree, board, tasks)) = self.load_state() {
            if let Some(col) = board.columns.get(self.selected_col) {
                let col_tasks: Vec<&(String, TaskPayload)> =
                    tasks.iter().filter(|(_, t)| t.column_id == col.id).collect();
                if let Some((task_id, task_snap)) = col_tasks.get(self.selected_task_idx) {
                    let next_col_idx = (self.selected_col + 1) % board.columns.len();
                    let next_col = &board.columns[next_col_idx];
                    let mut updated = task_snap.clone();
                    updated.column_id = next_col.id.clone();
                    updated.updated_at = Utc::now().to_rfc3339();
                    if let Ok(new_hash) = self.cas.put_object(OBJ_TASK, &updated) {
                        tree.tasks.insert(task_id.clone(), new_hash.clone());
                        if self.cas.save_working_tree(&tree).is_ok() {
                            self.status_banner = format!(
                                "Moved {} -> {} (new BLAKE3 object {})",
                                task_id,
                                next_col.name,
                                &new_hash[..10]
                            );
                        }
                    }
                }
            }
        }
    }

    fn commit_working_tree(&mut self) {
        if let Ok((tree_hash, _)) = self.cas.load_working_tree() {
            let parents = self.cas.resolve_head_commit().into_iter().collect();
            let payload = CommitPayload {
                tree_hash: tree_hash.clone(),
                parents,
                author: "TUI Operator".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                message: "Snapshot committed from clibm-tui".to_string(),
            };
            if let Ok(commit_hash) = self.cas.put_object(OBJ_COMMIT, &payload) {
                if let Some(branch) = self.cas.current_branch() {
                    let bfile = self.cas.clibm_dir.join("refs").join("heads").join(branch);
                    let _ = fs::write(bfile, format!("{}\n", commit_hash));
                } else {
                    let _ = fs::write(self.cas.clibm_dir.join("HEAD"), format!("{}\n", commit_hash));
                }
                self.status_banner = format!(
                    "Committed tree {} -> commit {}",
                    &tree_hash[..8],
                    &commit_hash[..10]
                );
            }
        }
    }

    fn run_fsck(&mut self) {
        if let Ok((verified, total)) = self.cas.fsck() {
            self.status_banner = format!(
                "BLAKE3 fsck complete: {}/{} objects verified intact",
                verified, total
            );
        }
    }
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
        name: "clibm-tui".to_string(),
        version: "0.1.0".to_string(),
        description:
            "Ratatui & Crossterm interactive Terminal User Interface (TUI) for ongoing projects"
                .to_string(),
        enabled: true,
        hooks: vec!["interactive-tui".to_string(), "on-board-move".to_string()],
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
        "Registered plugin 'clibm-tui' in global CLIBM registry ({})",
        reg_file.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--contract") {
        println!(
            "{{\"contract\":\"CLIBM_PLUGIN_CONTRACT_V1\",\"name\":\"clibm-tui\",\"version\":\"0.1.0\"}}"
        );
        return Ok(());
    }

    if let Some(pos) = args.iter().position(|a| a == "--register") {
        let custom_bin = args.get(pos + 1).cloned();
        return register_plugin_globally(custom_bin);
    }

    let cwd = env::current_dir()?;
    let cas = ProjectCas::discover_ongoing(&cwd)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(cas);

    loop {
        terminal.draw(|f| {
            let root_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(12),
                    Constraint::Length(7),
                    Constraint::Length(3),
                ])
                .split(f.area());

            if let Ok((tree_hash, _, board, tasks)) = app.load_state() {
                let branch = app
                    .cas
                    .current_branch()
                    .unwrap_or_else(|| "detached".to_string());

                // 1. Header Bar (Monochrome)
                let header = Paragraph::new(Line::from(vec![
                    Span::styled(
                        " CLIBM-TUI v0.1.0 ",
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(
                        "  Project: {}  |  Branch: {}  |  Tree: {}  |  Board: {}",
                        app.cas.project_root.display(),
                        branch,
                        &tree_hash[..12.min(tree_hash.len())],
                        board.name
                    )),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Ongoing Project TUI :: Ratatui + Crossterm"),
                );
                f.render_widget(header, root_chunks[0]);

                // 2. Kanban Columns Row
                let col_constraints: Vec<Constraint> = board
                    .columns
                    .iter()
                    .map(|_| Constraint::Ratio(1, board.columns.len().max(1) as u32))
                    .collect();
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(col_constraints)
                    .split(root_chunks[1]);

                let mut active_task: Option<TaskPayload> = None;

                for (idx, col) in board.columns.iter().enumerate() {
                    let col_tasks: Vec<&(String, TaskPayload)> =
                        tasks.iter().filter(|(_, t)| t.column_id == col.id).collect();
                    let is_col_selected = idx == app.selected_col;

                    let items: Vec<ListItem> = col_tasks
                        .iter()
                        .enumerate()
                        .map(|(t_idx, (tid, t))| {
                            let is_selected = is_col_selected && t_idx == app.selected_task_idx;
                            if is_selected {
                                active_task = Some(t.clone());
                            }
                            let prefix = if is_selected { "▶ " } else { "  " };
                            let style = if is_selected {
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(Color::White)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Gray)
                            };
                            ListItem::new(format!(
                                "{}{} [{}] {}",
                                prefix,
                                tid,
                                t.priority.to_uppercase(),
                                t.title
                            ))
                            .style(style)
                        })
                        .collect();

                    let border_style = if is_col_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let list = List::new(items).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border_style)
                            .title(format!(" {} ({}) ", col.name, col_tasks.len())),
                    );
                    f.render_widget(list, col_chunks[idx]);
                }

                // 3. Task Inspector Pane
                let inspector_text = if let Some(t) = active_task {
                    format!(
                        "Task: {} — {}\nPriority: {} | Assignee: {} | Depends On: {:?} | Tags: {:?}\nDescription: {}",
                        t.id,
                        t.title,
                        t.priority.to_uppercase(),
                        t.assignee.unwrap_or_else(|| "unassigned".to_string()),
                        t.depends_on,
                        t.tags,
                        t.description
                    )
                } else {
                    "Select a task using [h/l] and [j/k] to inspect its canonical payload and dependencies.".to_string()
                };

                let inspector = Paragraph::new(inspector_text)
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Selected Task Inspector (CAS Object)"),
                    );
                f.render_widget(inspector, root_chunks[2]);
            }

            // 4. Status & Keybindings Footer
            let footer = Paragraph::new(app.status_banner.as_str())
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("CLIBM_PLUGIN_CONTRACT_V1 :: Status"),
                );
            f.render_widget(footer, root_chunks[3]);
        })?;

        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('l') | KeyCode::Right => {
                            app.selected_col = (app.selected_col + 1) % 5;
                            app.selected_task_idx = 0;
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            app.selected_col = if app.selected_col == 0 {
                                4
                            } else {
                                app.selected_col - 1
                            };
                            app.selected_task_idx = 0;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.selected_task_idx = app.selected_task_idx.saturating_add(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.selected_task_idx = app.selected_task_idx.saturating_sub(1);
                        }
                        KeyCode::Char('m') => app.move_selected_task_forward(),
                        KeyCode::Char('c') => app.commit_working_tree(),
                        KeyCode::Char('f') => app.run_fsck(),
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
