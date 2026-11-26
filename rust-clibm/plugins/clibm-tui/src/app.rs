//! State and actions of the TUI. Every read and write goes through `clibm_core`.

use crate::form::{Form, FormOutcome};
use anyhow::Result;
use clibm_core::engine::{ClibmEngine, ProjectSummary, WorkspaceSummary};
use clibm_core::models::{KanbanBoardPayload, TaskPayload, WorkspacePayload, OBJ_TASK, OBJ_WORKSPACE};
use crossterm::event::{KeyCode, KeyEvent};
use std::time::Instant;

pub const KEYS_HELP: &str = "[h/l] column  [j/k] task  [m/b] move right/left  [w] workspaces & projects  \
[ [ / ] ] project  [W] workspace  [c] commit  [f] fsck  [r] reload  [q] quit";
pub const PICKER_HELP: &str =
    "[Enter] switch  [n] new  [e] edit  [d] delete  [Tab/h/l] pane  [j/k] select  [Esc] back";

/// What the board screen shows; reloaded after each action and every few seconds.
pub struct View {
    /// Short hash of the HEAD commit (latest entry of the history).
    pub head: String,
    pub tree_hash: String,
    pub dirty: bool,
    pub workspace_label: String,
    pub project_label: String,
    pub board: KanbanBoardPayload,
    pub tasks: Vec<TaskPayload>,
}

impl View {
    pub fn column_tasks(&self, col_idx: usize) -> Vec<&TaskPayload> {
        match self.board.columns.get(col_idx) {
            Some(col) => self.tasks.iter().filter(|t| t.column_id == col.id).collect(),
            None => Vec::new(),
        }
    }
}

/// Every workspace and project, for the picker.
#[derive(Default)]
pub struct Catalog {
    /// Ordered by name, as `clibm workspace list`.
    pub workspaces: Vec<WorkspaceSummary>,
    /// Ordered by workspace then project ID.
    pub projects: Vec<ProjectSummary>,
}

impl Catalog {
    pub fn projects_of(&self, ws_id: &str) -> Vec<&ProjectSummary> {
        self.projects.iter().filter(|p| p.workspace_id == ws_id).collect()
    }

    fn active_workspace(&self) -> Option<&WorkspaceSummary> {
        self.workspaces.iter().find(|w| w.active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Workspaces,
    Projects,
}

/// Picker selection, kept by ID so the periodic reload does not move it.
#[derive(Debug, Clone)]
pub struct Picker {
    pub focus: Pane,
    pub ws_id: Option<String>,
    pub prj_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormKind {
    NewWorkspace,
    EditWorkspace(String),
    /// New project in this workspace.
    NewProject(String),
    EditProject(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeleteTarget {
    Workspace { id: String, name: String, projects: usize, tasks: usize },
    Project { id: String, code: String, name: String, tasks: usize },
}

pub enum Mode {
    Board,
    Picker,
    Form { kind: FormKind, form: Form },
    /// `choices` are `(id, label)` of where the tasks can go; empty when nothing has to move.
    Confirm { target: DeleteTarget, choices: Vec<(String, String)>, pick: usize },
}

pub struct App {
    pub engine: ClibmEngine,
    pub view: Option<View>,
    pub catalog: Catalog,
    pub mode: Mode,
    pub picker: Picker,
    pub selected_col: usize,
    pub selected_task: usize,
    pub status: String,
    pub last_reload: Instant,
}

pub fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

fn commit_suffix(commit: &Option<String>) -> String {
    commit
        .as_deref()
        .map(|c| format!(" (commit {})", &c[..10.min(c.len())]))
        .unwrap_or_default()
}

impl App {
    pub fn new(engine: ClibmEngine) -> Self {
        let mut app = Self {
            engine,
            view: None,
            catalog: Catalog::default(),
            mode: Mode::Board,
            picker: Picker { focus: Pane::Projects, ws_id: None, prj_id: None },
            selected_col: 0,
            selected_task: 0,
            status: "Ready - press [w] to switch or manage workspaces and projects".to_string(),
            last_reload: Instant::now(),
        };
        app.reload();
        app
    }

    fn load_view(&self) -> Result<View> {
        let (tree_hash, tree) = self.engine.load_working_tree()?;
        let project = self.engine.active_project(&tree)?;
        let board = self.engine.project_board(&tree, &project)?;
        let workspace_label = tree
            .workspaces
            .get(&tree.active_workspace_id)
            .and_then(|h| self.engine.cas.get_typed_payload::<WorkspacePayload>(h, OBJ_WORKSPACE).ok())
            .map(|w| w.name)
            .unwrap_or_else(|| tree.active_workspace_id.clone());
        let mut tasks = Vec::new();
        for hash in tree.tasks.values() {
            let task: TaskPayload = self.engine.cas.get_typed_payload(hash, OBJ_TASK)?;
            if task.project_id == project.id && task.board_id == board.id {
                tasks.push(task);
            }
        }
        let head = self
            .engine
            .cas
            .resolve_head_commit()?
            .map(|h| h[..12.min(h.len())].to_string())
            .unwrap_or_else(|| "none".to_string());
        Ok(View {
            head,
            tree_hash,
            dirty: self.engine.is_dirty().unwrap_or(false),
            workspace_label,
            project_label: format!("{} - {}", project.code, project.name),
            board,
            tasks,
        })
    }

    fn load_catalog(&self) -> Result<Catalog> {
        Ok(Catalog {
            workspaces: self.engine.list_workspaces()?,
            projects: self.engine.list_projects(true)?,
        })
    }

    pub fn reload(&mut self) {
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
        match self.load_catalog() {
            Ok(c) => self.catalog = c,
            Err(e) => self.status = format!("Could not load workspaces: {}", first_line(&e.to_string())),
        }
        self.fix_picker();
        self.last_reload = Instant::now();
    }

    pub fn column_count(&self) -> usize {
        self.view
            .as_ref()
            .map(|v| v.board.columns.len())
            .unwrap_or(0)
            .max(1)
    }

    pub fn selected(&self) -> Option<(&TaskPayload, usize)> {
        let view = self.view.as_ref()?;
        let tasks = view.column_tasks(self.selected_col);
        tasks.get(self.selected_task).map(|t| (*t, self.selected_col))
    }

    // --- Board actions ---

    /// Moves the selected task one column to the right (`step = 1`) or left (`-1`).
    pub fn move_selected(&mut self, step: isize) {
        let Some((task, col_idx)) = self.selected() else {
            self.status = "No task selected".to_string();
            return;
        };
        let task_id = task.id.clone();
        let columns = &self.view.as_ref().expect("view exists").board.columns;
        let target_idx = col_idx as isize + step;
        if target_idx < 0 || target_idx as usize >= columns.len() {
            self.status = format!(
                "{} is already in the {} column",
                task_id,
                if step > 0 { "last" } else { "first" }
            );
            return;
        }
        let target = columns[target_idx as usize].clone();
        match self.engine.move_task(&task_id, &target.id, false) {
            Ok(res) => {
                self.status = format!("Moved {} to {}{}", task_id, target.name, commit_suffix(&res.commit));
                self.selected_col = target_idx as usize;
                self.reload();
                if let Some(view) = &self.view {
                    let tasks = view.column_tasks(self.selected_col);
                    self.selected_task = tasks.iter().position(|t| t.id == task_id).unwrap_or(0);
                }
            }
            Err(e) => {
                // e.g. a blocking dependency, a WIP limit or a plugin veto
                self.status = format!(
                    "Not moved: {} (use the CLI with --force to override)",
                    first_line(&e.to_string())
                );
            }
        }
    }

    pub fn commit(&mut self) {
        self.status = match self.engine.commit("Snapshot committed from clibm-tui", None) {
            Ok(Some(hash)) => format!("Committed {}", &hash[..10]),
            Ok(None) => "Nothing to commit: the working tree matches HEAD".to_string(),
            Err(e) => format!("Commit failed: {}", first_line(&e.to_string())),
        };
        self.reload();
    }

    pub fn fsck(&mut self) {
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

    /// Board changed (other project): start at the first column and task.
    fn reset_board_selection(&mut self) {
        self.selected_col = 0;
        self.selected_task = 0;
    }

    pub fn switch_project(&mut self, project_id: &str) {
        match self.engine.switch_project(project_id) {
            Ok((prj, commit)) => {
                self.status = format!("Active project: {} - {}{}", prj.code, prj.name, commit_suffix(&commit));
                self.reset_board_selection();
            }
            Err(e) => self.status = format!("Could not switch: {}", first_line(&e.to_string())),
        }
        self.reload();
    }

    pub fn switch_workspace(&mut self, workspace: Option<&str>) {
        match self.engine.switch_workspace(workspace) {
            Ok((ws, commit)) => {
                self.status = format!("Active workspace: {} ({}){}", ws.name, ws.id, commit_suffix(&commit));
                self.reset_board_selection();
            }
            Err(e) => self.status = format!("Could not switch: {}", first_line(&e.to_string())),
        }
        self.reload();
    }

    /// Switches to the next (`step = 1`) or previous (`-1`) project of the active workspace.
    pub fn cycle_project(&mut self, step: isize) {
        let Some(ws) = self.catalog.active_workspace() else {
            return;
        };
        let projects = self.catalog.projects_of(&ws.id);
        if projects.len() < 2 {
            self.status = format!("Workspace '{}' has only one project", ws.workspace.name);
            return;
        }
        let n = projects.len() as isize;
        let current = projects.iter().position(|p| p.active).unwrap_or(0) as isize;
        let next = projects[((current + step) % n + n) as usize % projects.len()].id.clone();
        self.switch_project(&next);
    }

    // --- Picker ---

    /// Keeps the picker selection on existing items: falls back to the active workspace
    /// and project, then to the first ones.
    fn fix_picker(&mut self) {
        let ws_ok = |id: &Option<String>| {
            id.as_ref()
                .map(|id| self.catalog.workspaces.iter().any(|w| &w.id == id))
                .unwrap_or(false)
        };
        if !ws_ok(&self.picker.ws_id) {
            self.picker.ws_id = self
                .catalog
                .active_workspace()
                .or(self.catalog.workspaces.first())
                .map(|w| w.id.clone());
        }
        let Some(ws_id) = self.picker.ws_id.clone() else {
            self.picker.prj_id = None;
            return;
        };
        let projects = self.catalog.projects_of(&ws_id);
        let prj_ok = self
            .picker
            .prj_id
            .as_ref()
            .map(|id| projects.iter().any(|p| &p.id == id))
            .unwrap_or(false);
        if !prj_ok {
            self.picker.prj_id = projects
                .iter()
                .find(|p| p.active)
                .or(projects.first())
                .map(|p| p.id.clone());
        }
    }

    pub fn picker_ws_index(&self) -> usize {
        self.picker
            .ws_id
            .as_ref()
            .and_then(|id| self.catalog.workspaces.iter().position(|w| &w.id == id))
            .unwrap_or(0)
    }

    pub fn picker_prj_index(&self) -> usize {
        let Some(ws_id) = &self.picker.ws_id else {
            return 0;
        };
        self.picker
            .prj_id
            .as_ref()
            .and_then(|id| self.catalog.projects_of(ws_id).iter().position(|p| &p.id == id))
            .unwrap_or(0)
    }

    pub fn open_picker(&mut self) {
        self.reload();
        self.picker.ws_id = self.catalog.active_workspace().map(|w| w.id.clone());
        self.picker.prj_id = self.catalog.projects.iter().find(|p| p.active).map(|p| p.id.clone());
        self.picker.focus = Pane::Projects;
        self.fix_picker();
        self.mode = Mode::Picker;
    }

    fn picker_move(&mut self, step: isize) {
        match self.picker.focus {
            Pane::Workspaces => {
                let n = self.catalog.workspaces.len();
                if n == 0 {
                    return;
                }
                let i = (self.picker_ws_index() as isize + step).clamp(0, n as isize - 1) as usize;
                let id = self.catalog.workspaces[i].id.clone();
                if self.picker.ws_id.as_deref() != Some(&id) {
                    self.picker.ws_id = Some(id);
                    self.picker.prj_id = None;
                    self.fix_picker();
                }
            }
            Pane::Projects => {
                let Some(ws_id) = self.picker.ws_id.clone() else {
                    return;
                };
                let projects = self.catalog.projects_of(&ws_id);
                if projects.is_empty() {
                    return;
                }
                let i = (self.picker_prj_index() as isize + step).clamp(0, projects.len() as isize - 1);
                self.picker.prj_id = Some(projects[i as usize].id.clone());
            }
        }
    }

    fn picker_ws(&self) -> Option<&WorkspaceSummary> {
        let id = self.picker.ws_id.as_ref()?;
        self.catalog.workspaces.iter().find(|w| &w.id == id)
    }

    fn picker_prj(&self) -> Option<&ProjectSummary> {
        let id = self.picker.prj_id.as_ref()?;
        self.catalog.projects.iter().find(|p| &p.id == id)
    }

    fn picker_enter(&mut self) {
        match self.picker.focus {
            Pane::Workspaces => {
                let Some(id) = self.picker.ws_id.clone() else { return };
                self.switch_workspace(Some(&id));
            }
            Pane::Projects => {
                let Some(id) = self.picker.prj_id.clone() else { return };
                self.switch_project(&id);
            }
        }
        self.mode = Mode::Board;
    }

    fn picker_new(&mut self) {
        let (kind, form) = match self.picker.focus {
            Pane::Workspaces => (
                FormKind::NewWorkspace,
                Form::new(" New workspace ")
                    .text("Name", "")
                    .text("Project code", "")
                    .text("Description", ""),
            ),
            Pane::Projects => {
                let Some(ws) = self.picker_ws() else { return };
                (
                    FormKind::NewProject(ws.id.clone()),
                    Form::new(format!(" New project in {} ", ws.workspace.name))
                        .text("Name", "")
                        .text("Code", "")
                        .text("Description", "")
                        .toggle("Switch to it", false),
                )
            }
        };
        self.mode = Mode::Form { kind, form };
    }

    fn picker_edit(&mut self) {
        let (kind, form) = match self.picker.focus {
            Pane::Workspaces => {
                let Some(ws) = self.picker_ws() else { return };
                (
                    FormKind::EditWorkspace(ws.id.clone()),
                    Form::new(format!(" Edit workspace {} ", ws.id))
                        .text("Name", &ws.workspace.name)
                        .text("Description", &ws.workspace.description),
                )
            }
            Pane::Projects => {
                let Some(p) = self.picker_prj() else { return };
                (
                    FormKind::EditProject(p.id.clone()),
                    Form::new(format!(" Edit project {} ", p.id))
                        .read_only("Code", &p.project.code)
                        .text("Name", &p.project.name)
                        .text("Description", &p.project.description),
                )
            }
        };
        self.mode = Mode::Form { kind, form };
    }

    fn picker_delete(&mut self) {
        match self.picker.focus {
            Pane::Workspaces => {
                let Some(ws) = self.picker_ws() else { return };
                if self.catalog.workspaces.len() <= 1 {
                    self.status = format!(
                        "Cannot delete {} ('{}'): it is the only workspace",
                        ws.id, ws.workspace.name
                    );
                    return;
                }
                let choices = if ws.tasks > 0 {
                    self.catalog
                        .workspaces
                        .iter()
                        .filter(|w| w.id != ws.id)
                        .map(|w| (w.id.clone(), format!("{} - {}", w.id, w.workspace.name)))
                        .collect()
                } else {
                    Vec::new()
                };
                let target = DeleteTarget::Workspace {
                    id: ws.id.clone(),
                    name: ws.workspace.name.clone(),
                    projects: ws.projects,
                    tasks: ws.tasks,
                };
                self.mode = Mode::Confirm { target, choices, pick: 0 };
            }
            Pane::Projects => {
                let Some(p) = self.picker_prj() else { return };
                if self.catalog.projects_of(&p.workspace_id).len() <= 1 {
                    self.status = format!(
                        "Cannot delete {} ({}): it is the only project of workspace '{}'",
                        p.id, p.project.code, p.workspace_name
                    );
                    return;
                }
                let choices = if p.tasks > 0 {
                    self.catalog
                        .projects
                        .iter()
                        .filter(|o| o.id != p.id)
                        .map(|o| {
                            (
                                o.id.clone(),
                                format!("{} {} - {} ({})", o.id, o.project.code, o.project.name, o.workspace_name),
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let target = DeleteTarget::Project {
                    id: p.id.clone(),
                    code: p.project.code.clone(),
                    name: p.project.name.clone(),
                    tasks: p.tasks,
                };
                self.mode = Mode::Confirm { target, choices, pick: 0 };
            }
        }
    }

    /// Runs a confirmed deletion.
    fn delete(&mut self, target: &DeleteTarget, move_to: Option<&str>) -> Result<String> {
        Ok(match target {
            DeleteTarget::Workspace { id, .. } => {
                let res = self.engine.delete_workspace(id, move_to)?;
                let mut s = format!("Deleted workspace {} ('{}')", res.id, res.name);
                if !res.moved_projects.is_empty() {
                    s.push_str(&format!(", moved {} project(s)", res.moved_projects.len()));
                }
                if !res.deleted_projects.is_empty() {
                    s.push_str(&format!(", deleted {} empty project(s)", res.deleted_projects.len()));
                }
                s + &commit_suffix(&res.commit)
            }
            DeleteTarget::Project { id, .. } => {
                let res = self.engine.delete_project(id, move_to)?;
                let mut s = format!("Deleted project {} ({})", res.id, res.code);
                if !res.moved_tasks.is_empty() {
                    s.push_str(&format!(", moved {} task(s)", res.moved_tasks.len()));
                }
                s + &commit_suffix(&res.commit)
            }
        })
    }

    /// Runs a submitted form. On success the dialog closes; on error it stays open.
    fn submit(&mut self, kind: &FormKind, form: &Form) -> Result<()> {
        let opt = |s: String| if s.is_empty() { None } else { Some(s) };
        match kind {
            FormKind::NewWorkspace => {
                let code = form.value("Project code");
                let (ws, _, commit) = self.engine.create_workspace(
                    &form.value("Name"),
                    opt(code).as_deref(),
                    &form.value("Description"),
                )?;
                self.status = format!("Created workspace {} ('{}'){}", ws.id, ws.name, commit_suffix(&commit));
                self.picker.ws_id = Some(ws.id);
                self.picker.prj_id = None;
                self.picker.focus = Pane::Workspaces;
                self.mode = Mode::Picker;
            }
            FormKind::EditWorkspace(id) => {
                let res = self.engine.edit_workspace(
                    id,
                    Some(form.value("Name")),
                    Some(form.value("Description")),
                )?;
                self.status = if res.changed {
                    format!("Updated workspace {}{}", res.id, commit_suffix(&res.commit))
                } else {
                    format!("Nothing changed in {}", res.id)
                };
                self.mode = Mode::Picker;
            }
            FormKind::NewProject(ws_id) => {
                let switch = form.checked("Switch to it");
                let (prj, _, commit) = self.engine.create_project_in(
                    Some(ws_id),
                    &form.value("Name"),
                    &form.value("Code"),
                    &form.value("Description"),
                    switch,
                )?;
                self.status = format!("Created project {} {} ('{}'){}", prj.id, prj.code, prj.name, commit_suffix(&commit));
                self.picker.ws_id = Some(prj.workspace_id);
                self.picker.prj_id = Some(prj.id);
                self.picker.focus = Pane::Projects;
                if switch {
                    self.reset_board_selection();
                    self.mode = Mode::Board;
                } else {
                    self.mode = Mode::Picker;
                }
            }
            FormKind::EditProject(id) => {
                let res = self.engine.edit_project(
                    id,
                    Some(form.value("Name")),
                    Some(form.value("Description")),
                )?;
                self.status = if res.changed {
                    format!("Updated project {}{}", res.id, commit_suffix(&res.commit))
                } else {
                    format!("Nothing changed in {}", res.id)
                };
                self.mode = Mode::Picker;
            }
        }
        self.reload();
        Ok(())
    }

    // --- Keys ---

    /// Handles one key press. Returns `true` when the TUI should quit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match std::mem::replace(&mut self.mode, Mode::Board) {
            Mode::Board => return self.board_key(key),
            Mode::Picker => {
                self.mode = Mode::Picker;
                self.picker_key(key);
            }
            Mode::Form { kind, mut form } => match form.handle_key(key) {
                FormOutcome::Continue => self.mode = Mode::Form { kind, form },
                FormOutcome::Cancel => {
                    self.status = "Cancelled".to_string();
                    self.mode = Mode::Picker;
                }
                FormOutcome::Submit => {
                    if let Err(e) = self.submit(&kind, &form) {
                        let msg = first_line(&e.to_string()).to_string();
                        self.status = format!("Not saved: {}", msg);
                        form.error = Some(msg);
                        self.mode = Mode::Form { kind, form };
                    }
                }
            },
            Mode::Confirm { target, choices, mut pick } => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let move_to = choices.get(pick).map(|(id, _)| id.clone());
                    match self.delete(&target, move_to.as_deref()) {
                        Ok(msg) => self.status = msg,
                        Err(e) => self.status = format!("Not deleted: {}", first_line(&e.to_string())),
                    }
                    self.reset_board_selection();
                    self.reload();
                    self.mode = Mode::Picker;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if pick + 1 < choices.len() {
                        pick += 1;
                    }
                    self.mode = Mode::Confirm { target, choices, pick };
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    pick = pick.saturating_sub(1);
                    self.mode = Mode::Confirm { target, choices, pick };
                }
                _ => {
                    self.status = "Not deleted".to_string();
                    self.mode = Mode::Picker;
                }
            },
        }
        false
    }

    fn board_key(&mut self, key: KeyEvent) -> bool {
        let columns = self.column_count();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('l') | KeyCode::Right => {
                self.selected_col = (self.selected_col + 1) % columns;
                self.selected_task = 0;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.selected_col = (self.selected_col + columns - 1) % columns;
                self.selected_task = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self
                    .view
                    .as_ref()
                    .map(|v| v.column_tasks(self.selected_col).len())
                    .unwrap_or(0);
                if self.selected_task + 1 < count {
                    self.selected_task += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_task = self.selected_task.saturating_sub(1);
            }
            KeyCode::Char('m') => self.move_selected(1),
            KeyCode::Char('b') => self.move_selected(-1),
            KeyCode::Char('c') => self.commit(),
            KeyCode::Char('f') => self.fsck(),
            KeyCode::Char('r') => {
                self.reload();
                self.status = "Reloaded".to_string();
            }
            KeyCode::Char('w') => self.open_picker(),
            KeyCode::Char(']') => self.cycle_project(1),
            KeyCode::Char('[') => self.cycle_project(-1),
            KeyCode::Char('W') => self.switch_workspace(None),
            _ => {}
        }
        false
    }

    fn picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Board,
            KeyCode::Tab | KeyCode::BackTab => {
                self.picker.focus = match self.picker.focus {
                    Pane::Workspaces => Pane::Projects,
                    Pane::Projects => Pane::Workspaces,
                }
            }
            KeyCode::Char('h') | KeyCode::Left => self.picker.focus = Pane::Workspaces,
            KeyCode::Char('l') | KeyCode::Right => self.picker.focus = Pane::Projects,
            KeyCode::Char('j') | KeyCode::Down => self.picker_move(1),
            KeyCode::Char('k') | KeyCode::Up => self.picker_move(-1),
            KeyCode::Enter => self.picker_enter(),
            KeyCode::Char('n') => self.picker_new(),
            KeyCode::Char('e') => self.picker_edit(),
            KeyCode::Char('d') => self.picker_delete(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clibm_core::models::TaskPriority;
    use crossterm::event::KeyModifiers;

    fn temp_repo(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("clibm-tui-{}-{}", label, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        ClibmEngine::open_exact(&dir).without_hooks().init("ws", "APP", "App").unwrap();
        dir
    }

    fn engine(dir: &std::path::Path) -> ClibmEngine {
        ClibmEngine::open_exact(dir).without_hooks()
    }

    fn press(app: &mut App, code: KeyCode) {
        assert!(!app.handle_key(KeyEvent::new(code, KeyModifiers::NONE)), "unexpected quit");
    }

    fn keys(app: &mut App, s: &str) {
        for c in s.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn clear_field(app: &mut App) {
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    }

    fn assert_fsck_clean(app: &mut App) {
        app.fsck();
        assert!(!app.status.contains("CORRUPTED") && !app.status.contains("legacy"), "{}", app.status);
    }

    fn add_task(dir: &std::path::Path, title: &str) {
        engine(dir)
            .create_task(title, "", TaskPriority::Medium, "todo", None, None, vec![], vec![], false)
            .unwrap();
    }

    #[test]
    fn view_follows_the_board_and_moves_use_core_rules() {
        let dir = temp_repo("moves");
        let e = engine(&dir);
        let mk = |title: &str, col: &str, deps: Vec<String>| {
            e.create_task(title, "", TaskPriority::Medium, col, None, None, vec![], deps, false)
                .unwrap();
        };
        mk("Base", "todo", vec![]);
        mk("Blocked", "todo", vec!["APP-101".into()]);

        let mut app = App::new(engine(&dir));
        let view = app.view.as_ref().expect("view loads");
        assert_eq!(view.board.columns.len(), 5, "columns come from the board");
        assert_eq!(view.column_tasks(1).len(), 2, "both tasks are in todo");

        // Select APP-102 in "todo" and try to move it to in-progress: blocked by APP-101.
        app.selected_col = 1;
        app.selected_task = 1;
        press(&mut app, KeyCode::Char('m'));
        assert!(app.status.starts_with("Not moved"), "{}", app.status);

        // APP-101 moves fine and the selection follows it.
        app.selected_task = 0;
        press(&mut app, KeyCode::Char('m'));
        assert!(app.status.starts_with("Moved APP-101"), "{}", app.status);
        assert_eq!(app.selected_col, 2);

        assert_fsck_clean(&mut app);
        assert!(app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)), "q quits from the board");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quick_keys_switch_projects_and_workspaces() {
        let dir = temp_repo("switch");
        let e = engine(&dir);
        e.create_project("Jobs", "JOB", "", false).unwrap();
        e.create_workspace("backend", Some("BE"), "").unwrap();

        let mut app = App::new(engine(&dir));
        press(&mut app, KeyCode::Char(']'));
        assert!(app.view.as_ref().unwrap().project_label.starts_with("JOB"), "{}", app.status);
        press(&mut app, KeyCode::Char('['));
        assert!(app.view.as_ref().unwrap().project_label.starts_with("APP"), "{}", app.status);

        // Workspaces cycle by name: backend comes before ws.
        press(&mut app, KeyCode::Char('W'));
        let view = app.view.as_ref().unwrap();
        assert_eq!(view.workspace_label, "backend");
        assert!(view.project_label.starts_with("BE"));
        // backend has one project: nothing to cycle.
        press(&mut app, KeyCode::Char(']'));
        assert!(app.status.contains("only one project"), "{}", app.status);

        // Switch back through the picker: workspaces pane, select "ws", Enter.
        press(&mut app, KeyCode::Char('w'));
        assert!(matches!(app.mode, Mode::Picker));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Board));
        assert_eq!(app.view.as_ref().unwrap().workspace_label, "ws");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn picker_creates_and_edits_workspaces_and_projects() {
        let dir = temp_repo("create");
        let mut app = App::new(engine(&dir));

        // New workspace from the workspaces pane.
        press(&mut app, KeyCode::Char('w'));
        press(&mut app, KeyCode::Char('h'));
        press(&mut app, KeyCode::Char('n'));
        keys(&mut app, "Ops");
        press(&mut app, KeyCode::Tab);
        keys(&mut app, "ops");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Picker), "{}", app.status);
        assert_eq!(app.catalog.workspaces.len(), 2);
        assert_eq!(app.picker.ws_id.as_deref(), Some("WS-02"));

        // New project in the highlighted (not active) workspace, without switching.
        press(&mut app, KeyCode::Char('l'));
        press(&mut app, KeyCode::Char('n'));
        keys(&mut app, "Infra");
        press(&mut app, KeyCode::Tab);
        keys(&mut app, "INF");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Picker), "{}", app.status);
        let infra = app.catalog.projects.iter().find(|p| p.project.code == "INF").unwrap();
        assert_eq!(infra.workspace_id, "WS-02");
        assert!(!infra.active);
        assert_eq!(app.picker.prj_id.as_deref(), Some(infra.id.as_str()));

        // Edit the project: the code is read-only, the name field has the focus.
        press(&mut app, KeyCode::Char('e'));
        clear_field(&mut app);
        keys(&mut app, "Infrastructure");
        press(&mut app, KeyCode::Enter);
        let infra = app.catalog.projects.iter().find(|p| p.project.code == "INF").unwrap();
        assert_eq!(infra.project.name, "Infrastructure");

        // A duplicate workspace name is refused and the form stays open with the error.
        press(&mut app, KeyCode::Char('h'));
        press(&mut app, KeyCode::Char('e'));
        clear_field(&mut app);
        keys(&mut app, "WS");
        press(&mut app, KeyCode::Enter);
        match &app.mode {
            Mode::Form { form, .. } => assert!(form.error.as_deref().unwrap_or("").contains("already used")),
            _ => panic!("form should stay open: {}", app.status),
        }
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::Picker));

        // Esc goes back to the board; the active project never changed.
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::Board));
        assert!(app.view.as_ref().unwrap().project_label.starts_with("APP"));
        assert_fsck_clean(&mut app);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn picker_deletes_with_a_move_target_and_refuses_the_last_items() {
        let dir = temp_repo("delete");
        add_task(&dir, "Keep me");
        engine(&dir).create_project("Jobs", "JOB", "", false).unwrap();
        let mut app = App::new(engine(&dir));

        // APP has a task: the dialog asks where it goes (the only other project, JOB).
        press(&mut app, KeyCode::Char('w'));
        assert_eq!(app.picker.prj_id.as_deref(), Some("PRJ-01"));
        press(&mut app, KeyCode::Char('d'));
        match &app.mode {
            Mode::Confirm { choices, .. } => assert_eq!(choices.len(), 1),
            _ => panic!("expected a confirmation: {}", app.status),
        }
        // Anything but y cancels.
        press(&mut app, KeyCode::Char('x'));
        assert!(matches!(app.mode, Mode::Picker));
        assert_eq!(app.catalog.projects.len(), 2);

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert!(app.status.starts_with("Deleted project PRJ-01"), "{}", app.status);
        assert_eq!(app.catalog.projects.len(), 1);
        assert_eq!(app.catalog.projects[0].tasks, 1, "the task moved to JOB");
        assert!(app.view.as_ref().unwrap().project_label.starts_with("JOB"));

        // Last project of the workspace, and last workspace: refused without a dialog.
        press(&mut app, KeyCode::Char('d'));
        assert!(app.status.contains("only project"), "{}", app.status);
        assert!(matches!(app.mode, Mode::Picker));
        press(&mut app, KeyCode::Char('h'));
        press(&mut app, KeyCode::Char('d'));
        assert!(app.status.contains("only workspace"), "{}", app.status);
        assert!(matches!(app.mode, Mode::Picker));

        // A second, empty workspace can be deleted with y (no target needed).
        engine(&dir).create_workspace("tmp", Some("TMP"), "").unwrap();
        app.reload();
        app.picker.ws_id = Some("WS-02".to_string());
        press(&mut app, KeyCode::Char('d'));
        match &app.mode {
            Mode::Confirm { choices, .. } => assert!(choices.is_empty()),
            _ => panic!("expected a confirmation: {}", app.status),
        }
        press(&mut app, KeyCode::Char('y'));
        assert!(app.status.starts_with("Deleted workspace WS-02"), "{}", app.status);
        assert_eq!(app.catalog.workspaces.len(), 1);
        assert_fsck_clean(&mut app);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
