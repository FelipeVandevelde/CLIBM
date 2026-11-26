//! CLIBM-TUI — interactive terminal Kanban board for a CLIBM project.
//!
//! Run it inside a project with `clibm plugin run tui` (or `clibm-tui`).
//! Every read and write goes through `clibm_core`, so moves follow the same rules as
//! the CLI (dependencies, WIP limits, hooks, auto-commit). Press `w` to switch, create,
//! edit or delete workspaces and projects.

mod app;
mod form;
mod ui;

use anyhow::{bail, Result};
use app::{App, KEYS_HELP};
use clibm_core::engine::ClibmEngine;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env, io,
    time::Duration,
};

const PLUGIN_NAME: &str = "clibm-tui";
const PLUGIN_DESCRIPTION: &str = "Interactive Ratatui terminal UI for CLIBM projects";
/// Lifecycle events this plugin subscribes to by default.
const DEFAULT_HOOKS: &[&str] = &["interactive-tui"];

/// External changes (CLI, MCP, web) are picked up at this interval.
const RELOAD_EVERY: Duration = Duration::from_secs(2);

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        if app.last_reload.elapsed() >= RELOAD_EVERY {
            app.reload();
        }
        terminal.draw(|f| ui::draw(f, app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if app.handle_key(key) {
            return Ok(());
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
