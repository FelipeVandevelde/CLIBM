//! Drawing: the board screen plus the picker, form and delete-confirmation dialogs.

use crate::app::{App, DeleteTarget, Mode, Pane, KEYS_HELP, PICKER_HELP};
use crate::form::{FieldKind, Form};
use clibm_core::models::TaskPayload;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

fn selected_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD)
}

fn border(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// A rectangle of `width` x `height` cells centered in `area` (clamped to it).
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

pub fn draw(f: &mut Frame, app: &App) {
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
            Span::styled(format!(" CLIBM-TUI v{} ", env!("CARGO_PKG_VERSION")), selected_style()),
            Span::raw(format!(
                "  {} › {}  |  HEAD {}  |  tree {}{}  |  {}",
                view.workspace_label,
                view.project_label,
                view.head,
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
                    let style = if selected { selected_style() } else { Style::default().fg(Color::Gray) };
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
                    .border_style(border(col_selected))
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

    let help = match app.mode {
        Mode::Board => KEYS_HELP,
        Mode::Picker => PICKER_HELP,
        Mode::Form { .. } => "[Tab] next field  [Space] toggle  [Ctrl+U] clear  [Enter] save  [Esc] cancel",
        Mode::Confirm { .. } => "[y] delete  [j/k] choose target  [any other key] cancel",
    };
    let footer = Paragraph::new(app.status.as_str())
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(format!(" {} ", help)));
    f.render_widget(footer, rows[3]);

    match &app.mode {
        Mode::Board => {}
        Mode::Picker => draw_picker(f, app),
        Mode::Form { form, .. } => {
            draw_picker(f, app);
            draw_form(f, form);
        }
        Mode::Confirm { target, choices, pick } => {
            draw_picker(f, app);
            draw_confirm(f, target, choices, *pick);
        }
    }
}

fn draw_picker(f: &mut Frame, app: &App) {
    let full = f.area();
    // Leave the footer (status line) visible below the dialog.
    let area = centered(
        Rect { height: full.height.saturating_sub(3), ..full },
        (full.width * 9 / 10).max(40),
        (full.height * 7 / 10).max(12),
    );
    f.render_widget(Clear, area);
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(border(true))
        .title(" Workspaces & projects ");
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(inner);
    let in_picker = matches!(app.mode, Mode::Picker);

    let ws_items: Vec<ListItem> = app
        .catalog
        .workspaces
        .iter()
        .map(|w| {
            ListItem::new(format!(
                "{} {} {}  ({} prj, {} tasks)",
                if w.active { "*" } else { " " },
                w.id,
                w.workspace.name,
                w.projects,
                w.tasks
            ))
        })
        .collect();
    let ws_focus = app.picker.focus == Pane::Workspaces;
    let ws_list = List::new(ws_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border(ws_focus && in_picker))
                .title(" Workspaces "),
        )
        .highlight_style(if ws_focus { selected_style() } else { Style::default().add_modifier(Modifier::BOLD) })
        .highlight_symbol("> ");
    let mut ws_state = ListState::default().with_selected(Some(app.picker_ws_index()));
    f.render_stateful_widget(ws_list, panes[0], &mut ws_state);

    let ws_name = app
        .picker
        .ws_id
        .as_ref()
        .and_then(|id| app.catalog.workspaces.iter().find(|w| &w.id == id))
        .map(|w| w.workspace.name.clone())
        .unwrap_or_default();
    let projects = app
        .picker
        .ws_id
        .as_ref()
        .map(|id| app.catalog.projects_of(id))
        .unwrap_or_default();
    let prj_items: Vec<ListItem> = projects
        .iter()
        .map(|p| {
            ListItem::new(format!(
                "{} {} {} - {}  ({} tasks)",
                if p.active { "*" } else { " " },
                p.id,
                p.project.code,
                p.project.name,
                p.tasks
            ))
        })
        .collect();
    let prj_focus = app.picker.focus == Pane::Projects;
    let prj_list = List::new(prj_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border(prj_focus && in_picker))
                .title(format!(" Projects of {} ", ws_name)),
        )
        .highlight_style(if prj_focus { selected_style() } else { Style::default().add_modifier(Modifier::BOLD) })
        .highlight_symbol("> ");
    let mut prj_state = ListState::default().with_selected(Some(app.picker_prj_index()));
    f.render_stateful_widget(prj_list, panes[1], &mut prj_state);
}

fn draw_form(f: &mut Frame, form: &Form) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let focused = i == form.focus;
        let label = Span::styled(
            format!("{} {:<14}", if focused { ">" } else { " " }, format!("{}:", field.label)),
            if focused { Style::default().add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) },
        );
        let mut spans = vec![label];
        match field.kind {
            FieldKind::Toggle(on) => spans.push(Span::raw(if on { "[x]" } else { "[ ]" })),
            FieldKind::ReadOnly => spans.push(Span::styled(
                format!("{} (cannot change)", field.value),
                Style::default().fg(Color::DarkGray),
            )),
            FieldKind::Text if focused => {
                // Show the cursor as a highlighted cell.
                let chars: Vec<char> = field.value.chars().collect();
                let before: String = chars[..field.cursor].iter().collect();
                let at: String = chars.get(field.cursor).map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
                let after: String = chars.get(field.cursor + 1..).map(|s| s.iter().collect()).unwrap_or_default();
                spans.push(Span::raw(before));
                spans.push(Span::styled(at, selected_style()));
                spans.push(Span::raw(after));
            }
            FieldKind::Text => spans.push(Span::raw(field.value.clone())),
        }
        lines.push(Line::from(spans));
    }
    if let Some(err) = &form.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(err.clone(), Style::default().fg(Color::Red))));
    }
    let area = centered(f.area(), 70, lines.len() as u16 + 4);
    f.render_widget(Clear, area);
    let p = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border(true))
            .title(form.title.clone()),
    );
    f.render_widget(p, area);
}

fn draw_confirm(f: &mut Frame, target: &DeleteTarget, choices: &[(String, String)], pick: usize) {
    let mut lines: Vec<Line> = Vec::new();
    let bold = Style::default().add_modifier(Modifier::BOLD);
    match target {
        DeleteTarget::Workspace { id, name, projects, tasks } => {
            lines.push(Line::from(Span::styled(format!("Delete workspace {} ('{}')?", id, name), bold)));
            if *tasks > 0 {
                lines.push(Line::from(format!(
                    "Its {} project(s) hold {} task(s). Move the projects, whole, to:",
                    projects, tasks
                )));
            } else {
                lines.push(Line::from(format!("Its {} empty project(s) and their boards are deleted too.", projects)));
            }
        }
        DeleteTarget::Project { id, code, name, tasks } => {
            lines.push(Line::from(Span::styled(format!("Delete project {} {} ('{}') and its board?", id, code, name), bold)));
            if *tasks > 0 {
                lines.push(Line::from(format!("Its {} task(s) keep their IDs and move to:", tasks)));
            } else {
                lines.push(Line::from("It has no tasks."));
            }
        }
    }
    for (i, (_, label)) in choices.iter().enumerate() {
        if i == pick {
            lines.push(Line::from(Span::styled(format!("> {}", label), selected_style())));
        } else {
            lines.push(Line::from(format!("  {}", label)));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "History is kept: `clibm restore` brings it back.",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled("[y] delete   [any other key] cancel", bold)));
    let area = centered(f.area(), 80, lines.len() as u16 + 2);
    f.render_widget(Clear, area);
    let p = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" Delete "),
    );
    f.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clibm_core::engine::ClibmEngine;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    fn screen(app: &App, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..h {
            for x in 0..w {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn press(app: &mut App, c: KeyCode) {
        app.handle_key(KeyEvent::new(c, KeyModifiers::NONE));
    }

    #[test]
    fn every_mode_renders_at_normal_and_tiny_sizes() {
        let dir = std::env::temp_dir().join(format!("clibm-tui-ui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let e = ClibmEngine::open_exact(&dir).without_hooks();
        e.init("ws", "APP", "App").unwrap();
        e.create_project("Jobs", "JOB", "", false).unwrap();
        let mut app = App::new(ClibmEngine::open_exact(&dir).without_hooks());

        let board = screen(&app, 120, 40);
        assert!(board.contains("ws › APP - App"), "{}", board);

        press(&mut app, KeyCode::Char('w'));
        let picker = screen(&app, 120, 40);
        assert!(picker.contains("Workspaces & projects"), "{}", picker);
        assert!(picker.contains("JOB - Jobs"), "{}", picker);

        press(&mut app, KeyCode::Char('e'));
        let form = screen(&app, 120, 40);
        assert!(form.contains("Edit project PRJ-01"), "{}", form);
        assert!(form.contains("(cannot change)"), "{}", form);
        screen(&app, 30, 10);
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('d'));
        let confirm = screen(&app, 120, 40);
        assert!(confirm.contains("Delete project PRJ-02 JOB"), "{}", confirm);
        screen(&app, 30, 10);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
