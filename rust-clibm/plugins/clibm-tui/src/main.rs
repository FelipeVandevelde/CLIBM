//! CLIBM-TUI — interactive terminal Kanban board for a CLIBM project.
//!
//! Run it inside a project with `clibm plugin run tui` (or `clibm-tui`).
//! Every read and write goes through `clibm_core`, so moves follow the same rules as
//! the CLI (dependencies, WIP limits, hooks, auto-commit).

use anyhow::{bail, Result};
use clibm_core::engine::ClibmEngine;
use clibm_core::models::{KanbanBoardPayload, TaskPayload, OBJ_TASK};
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
use std::{
    env, io,
    time::{Duration, Instant},
};

const PLUGIN_NAME: &str = "clibm-tui";
const PLUGIN_DESCRIPTION: &str = "Interactive Ratatui terminal UI for CLIBM projects";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["interactive-tui"];

const KEYS_HELP: &str =
    "[h/l] column  [j/k] task  [m] move right  [b] move left  [c] commit  [f] fsck  [r] reload  [q] quit";
/// External changes (CLI, MCP, web) are picked up at this interval.
const RELOAD_EVERY: Duration = Duration::from_secs(2);

/// What the screen shows; reloaded after each action and every few seconds.
struct View {
    branch: String,
    tree_hash: String,
    dirty: bool,
    project_label: String,
    board: KanbanBoardPayload,
    tasks: Vec<TaskPayload>,
}

impl View {
    fn column_tasks(&self, col_idx: usize) -> Vec<&TaskPayload> {
        match self.board.columns.get(col_idx) {
            Some(col) => self.tasks.iter().filter(|t| t.column_id == col.id).collect(),
            None => Vec::new(),
        }
    }
}

struct App {
    engine: ClibmEngine,
    view: Option<View>,
    selected_col: usize,
    selected_task: usize,
    status: String,
    last_reload: Instant,
}

impl App {
    fn new(engine: ClibmEngine) -> Self {
        let mut app = Self {
            engine,
            view: None,
            selected_col: 0,
            selected_task: 0,
            status: format!("Ready - {}", KEYS_HELP),
            last_reload: Instant::now(),
        };
        app.reload();
        app
    }

    fn load_view(&self) -> Result<View> {
        let (tree_hash, tree) = self.engine.load_working_tree()?;
        let project = self.engine.active_project(&tree)?;
        let board = self.engine.project_board(&tree, &project)?;
        let mut tasks = Vec::new();
        for hash in tree.tasks.values() {
            let task: TaskPayload = self.engine.cas.get_typed_payload(hash, OBJ_TASK)?;
            if task.project_id == project.id && task.board_id == board.id {
                tasks.push(task);
            }
        }
        let branch = self
            .engine
            .cas
            .current_branch()?
            .unwrap_or_else(|| "detached".to_string());
        Ok(View {
            branch,
            tree_hash,
            dirty: self.engine.is_dirty().unwrap_or(false),
            project_label: format!("{} - {}", project.code, project.name),
            board,
            tasks,
        })
    }

    fn reload(&mut self) {
        match self.load_view() {
            Ok(view) => {
                let columns = view.board.columns.len().max(1);
                self.selected_col = self.selected_col.min(columns - 1);
                let count = view.column_tasks(self.selected_col).len();
                self.selected_task = self.selected_task.min(count.saturating_sub(1));
                self.view = Some(view);
            }
            Err(e) => self.status = format!("Could not load the project: {}", e),
        }
        self.last_reload = Instant::now();
    }

    fn column_count(&self) -> usize {
        self.view
            .as_ref()
            .map(|v| v.board.columns.len())
            .unwrap_or(0)
            .max(1)
    }

    fn selected(&self) -> Option<(&TaskPayload, usize)> {
        let view = self.view.as_ref()?;
        let tasks = view.column_tasks(self.selected_col);
        tasks.get(self.selected_task).map(|t| (*t, self.selected_col))
    }

    /// Moves the selected task one column to the right (`step = 1`) or left (`-1`).
    fn move_selected(&mut self, step: isize) {
        let Some((task, col_idx)) = self.selected() else {
            self.status = "No task selected".to_string();
            return;
        };
        let task_id = task.id.clone();
        let columns = &self.view.as_ref().expect("view exists").board.columns;
        let target_idx = col_idx as isize + step;
        if target_idx < 0 || target_idx as usize >= columns.len() {
            self.status = format!("{} is already in the {} column", task_id, if step > 0 { "last" } else { "first" });
            return;
        }
        let target = columns[target_idx as usize].clone();
        match self.engine.move_task(&task_id, &target.id, false) {
            Ok(res) => {
                self.status = format!(
                    "Moved {} to {}{}",
                    task_id,
                    target.name,
                    res.commit
                        .as_deref()
                        .map(|c| format!(" (commit {})", &c[..10.min(c.len())]))
                        .unwrap_or_default()
                );
                self.selected_col = target_idx as usize;
                self.reload();
                if let Some(view) = &self.view {
                    let tasks = view.column_tasks(self.selected_col);
                    self.selected_task = tasks.iter().position(|t| t.id == task_id).unwrap_or(0);
                }
            }
            Err(e) => {
                // e.g. a blocking dependency, a WIP limit or a plugin veto
                self.status = format!("Not moved: {} (use the CLI with --force to override)", first_line(&e.to_string()));
            }
        }
    }

    fn commit(&mut self) {
        self.status = match self.engine.commit("Snapshot committed from clibm-tui", None) {
            Ok(Some(hash)) => format!("Committed {}", &hash[..10]),
            Ok(None) => "Nothing to commit: the working tree matches HEAD".to_string(),
            Err(e) => format!("Commit failed: {}", first_line(&e.to_string())),
        };
        self.reload();
    }

    fn fsck(&mut self) {
        self.status = match self.engine.cas.fsck() {
            Ok(r) => {
                let mut s = format!("fsck: {}/{} objects verified", r.verified_objects, r.total_objects);
                if !r.legacy_hashes.is_empty() {
                    s.push_str(&format!(", {} legacy", r.legacy_hashes.len()));
                }
                if !r.corrupted_hashes.is_empty() {
                    s.push_str(&format!(", {} CORRUPTED", r.corrupted_hashes.len()));
                }
                s
            }
            Err(e) => format!("fsck failed: {}", e),
        };
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(f.area());

    if let Some(view) = &app.view {
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" CLIBM-TUI v{} ", env!("CARGO_PKG_VERSION")),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  {}  |  branch {}  |  tree {}{}  |  {}",
                view.project_label,
                view.branch,
                &view.tree_hash[..12.min(view.tree_hash.len())],
                if view.dirty { " (uncommitted)" } else { "" },
                view.board.name
            )),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, rows[0]);

        let n = view.board.columns.len().max(1) as u32;
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(view.board.columns.iter().map(|_| Constraint::Ratio(1, n)).collect::<Vec<_>>())
            .split(rows[1]);

        let mut inspected: Option<&TaskPayload> = None;
        for (idx, col) in view.board.columns.iter().enumerate() {
            let tasks = view.column_tasks(idx);
            let col_selected = idx == app.selected_col;
            let items: Vec<ListItem> = tasks
                .iter()
                .enumerate()
                .map(|(t_idx, t)| {
                    let selected = col_selected && t_idx == app.selected_task;
                    if selected {
                        inspected = Some(t);
                    }
                    let style = if selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    ListItem::new(format!(
                        "{}{} [{}] {}",
                        if selected { "> " } else { "  " },
                        t.id,
                        t.priority.as_str().to_uppercase(),
                        t.title
                    ))
                    .style(style)
                })
                .collect();
            let count = match col.wip_limit {
                Some(limit) => format!("{}/{}", tasks.len(), limit),
                None => tasks.len().to_string(),
            };
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if col_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    })
                    .title(format!(" {} ({}) ", col.name, count)),
            );
            f.render_widget(list, col_chunks[idx]);
        }

        let details = match inspected {
            Some(t) => format!(
                "{} - {}\nPriority: {} | Assignee: {} | Due: {} | Depends on: {} | Tags: {}\n{}",
                t.id,
                t.title,
                t.priority.as_str().to_uppercase(),
                t.assignee.as_deref().unwrap_or("unassigned"),
                t.due_date.as_deref().unwrap_or("-"),
                if t.depends_on.is_empty() { "-".to_string() } else { t.depends_on.join(", ") },
                if t.tags.is_empty() { "-".to_string() } else { t.tags.join(", ") },
                t.description
            ),
            None => "Select a task with [h/l] and [j/k].".to_string(),
        };
        let inspector = Paragraph::new(details)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title(" Task "));
        f.render_widget(inspector, rows[2]);
    }

    let footer = Paragraph::new(app.status.as_str())
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(format!(" {} ", KEYS_HELP)));
    f.render_widget(footer, rows[3]);
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        if app.last_reload.elapsed() >= RELOAD_EVERY {
            app.reload();
        }
        terminal.draw(|f| draw(f, app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let columns = app.column_count();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('l') | KeyCode::Right => {
                app.selected_col = (app.selected_col + 1) % columns;
                app.selected_task = 0;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                app.selected_col = (app.selected_col + columns - 1) % columns;
                app.selected_task = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = app
                    .view
                    .as_ref()
                    .map(|v| v.column_tasks(app.selected_col).len())
                    .unwrap_or(0);
                if app.selected_task + 1 < count {
                    app.selected_task += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.selected_task = app.selected_task.saturating_sub(1);
            }
            KeyCode::Char('m') => app.move_selected(1),
            KeyCode::Char('b') => app.move_selected(-1),
            KeyCode::Char('c') => app.commit(),
            KeyCode::Char('f') => app.fsck(),
            KeyCode::Char('r') => {
                app.reload();
                app.status = "Reloaded".to_string();
            }
            _ => {}
        }
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--manifest") {
        println!("{}", manifest_json(&args));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("clibm-tui - terminal Kanban board for the CLIBM project in the current folder\n\nKeys: {}", KEYS_HELP);
        return Ok(());
    }
    if let Some(unknown) = args.first() {
        eprintln!("Error: unknown argument '{}' (try --help)", unknown);
        std::process::exit(2);
    }

    let cwd = env::current_dir()?;
    let engine = ClibmEngine::open(&cwd);
    if !engine.cas.is_initialized() {
        bail!(
            "No CLIBM project (.clibm/) found in '{}' or its parent folders. Run `clibm init` first.",
            cwd.display()
        );
    }

    // Always give the terminal back, even if something panics.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(engine);
    let result = run(&mut terminal, &mut app);
    restore_terminal();
    let _ = terminal.show_cursor();
    result
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
    use clibm_core::models::TaskPriority;

    #[test]
    fn view_follows_the_board_and_moves_use_core_rules() {
        let dir = std::env::temp_dir().join(format!("clibm-tui-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = ClibmEngine::open_exact(&dir).without_hooks();
        engine.init("ws", "APP", "App").unwrap();
        let mk = |title: &str, col: &str, deps: Vec<String>| {
            engine
                .create_task(title, "", TaskPriority::Medium, col, None, None, vec![], deps, false)
                .unwrap();
        };
        mk("Base", "todo", vec![]);
        mk("Blocked", "todo", vec!["APP-101".into()]);

        let mut app = App::new(ClibmEngine::open_exact(&dir).without_hooks());
        let view = app.view.as_ref().expect("view loads");
        assert_eq!(view.board.columns.len(), 5, "columns come from the board");
        assert_eq!(view.column_tasks(1).len(), 2, "both tasks are in todo");

        // Select APP-102 in "todo" and try to move it to in-progress: blocked by APP-101.
        app.selected_col = 1;
        app.selected_task = 1;
        app.move_selected(1);
        assert!(app.status.starts_with("Not moved"), "{}", app.status);

        // APP-101 moves fine and the selection follows it.
        app.selected_task = 0;
        app.move_selected(1);
        assert!(app.status.starts_with("Moved APP-101"), "{}", app.status);
        assert_eq!(app.selected_col, 2);

        // fsck stays clean.
        app.fsck();
        assert!(!app.status.contains("CORRUPTED") && !app.status.contains("legacy"), "{}", app.status);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
