//! CLIBM MCP server plugin (`clibm-mcp`)
//!
//! Exposes a CLIBM project to AI agents and IDEs (Claude Desktop, Claude Code, Cursor,
//! Windsurf, custom agents) over the Model Context Protocol (`2024-11-05`).
//!
//! Features:
//!   - `transport-stdio`: JSON-RPC 2.0 over stdin/stdout (what AI clients launch).
//!   - `transport-http`: local HTTP endpoint (`POST http://127.0.0.1:<port>/mcp`).

#[cfg(not(any(feature = "transport-stdio", feature = "transport-http")))]
compile_error!(
    "Invalid build configuration: At least one transport feature must be enabled.\n\
     Use: --features transport-stdio, or --features transport-http, or default features."
);

mod adapter;
mod backend;
mod config;
mod error;
mod protocol;
mod transport;

use adapter::McpAdapter;
use anyhow::{bail, Result};
use config::{McpConfig, TransportMode};
use std::env;
#[cfg(all(feature = "transport-stdio", feature = "transport-http"))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const USAGE: &str = "USAGE:
  clibm-mcp [OPTIONS]
  clibm plugin run mcp -- [OPTIONS]

OPTIONS:
  --stdio          JSON-RPC 2.0 over stdin/stdout (default)
  --http           Local HTTP endpoint: POST http://127.0.0.1:<port>/mcp
  --both           stdio in the foreground and HTTP in the background
  -p, --port <P>   Port for HTTP (default 7879)
  --root <PATH>    Project folder containing .clibm (default: current folder or CLIBM_PROJECT_ROOT)
  --read-only      Refuse tools that change the project
  --print-config   Print the MCP client configuration (.mcp.json snippet)
  -V, --version    Print the version
  -h, --help       Print this help";

fn print_help() {
    println!(
        "clibm-mcp {} - Model Context Protocol server for CLIBM projects\n\n{}\n\n\
Setup guide: https://github.com/FelipeVandevelde/CLIBM/blob/main/rust-clibm/plugins/clibm-mcp/README.md",
        env!("CARGO_PKG_VERSION"),
        USAGE
    );
}

/// The `mcpServers` entry AI clients need (`.mcp.json`, Claude Desktop, Cursor, ...).
fn client_config_json() -> String {
    let config = serde_json::json!({
        "mcpServers": {
            "clibm": {
                "command": "clibm-mcp",
                "args": ["--stdio"]
            }
        }
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

/// `--manifest [--executable <path>] [--hooks a,b]`: prints this plugin's registry entry
/// (one JSON line) for installers. The hooks name the compiled-in transports.
fn manifest_json(args: &[String]) -> String {
    let value_after = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let mut default_hooks: Vec<String> = Vec::new();
    if cfg!(feature = "transport-stdio") {
        default_hooks.push("mcp-stdio".to_string());
    }
    if cfg!(feature = "transport-http") {
        default_hooks.push("mcp-http".to_string());
    }
    let hooks: Vec<String> = match value_after("--hooks") {
        Some(csv) => csv
            .split(',')
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
            .collect(),
        None => default_hooks,
    };
    serde_json::json!({
        "name": "clibm-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Model Context Protocol (MCP) server that exposes a CLIBM project to AI agents",
        "enabled": true,
        "hooks": hooks,
        "executable": value_after("--executable"),
    })
    .to_string()
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--manifest") {
        println!("{}", manifest_json(&args));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("clibm-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--print-config") {
        println!("{}", client_config_json());
        return Ok(());
    }

    let config = match McpConfig::load_from_env_and_args(&args) {
        Ok(cfg) => cfg,
        Err(message) => {
            eprintln!("Error: {}\n\n{}", message, USAGE);
            std::process::exit(2);
        }
    };

    if matches!(config.transport, TransportMode::Http | TransportMode::Both) && !cfg!(feature = "transport-http") {
        bail!("This clibm-mcp build has no HTTP transport. Reinstall it with the 'http' or 'both' transport.");
    }
    if matches!(config.transport, TransportMode::Stdio | TransportMode::Both) && !cfg!(feature = "transport-stdio") {
        bail!("This clibm-mcp build has no stdio transport. Reinstall it with the 'stdio' or 'both' transport.");
    }

    let adapter = Arc::new(McpAdapter::new(config.clone()));

    match config.transport {
        TransportMode::Http => {
            #[cfg(feature = "transport-http")]
            return transport::http::HttpTransport::run(adapter, config.http_port, None);
            #[cfg(not(feature = "transport-http"))]
            unreachable!();
        }
        TransportMode::Stdio => {
            #[cfg(feature = "transport-stdio")]
            return transport::stdio::StdioTransport::run(adapter);
            #[cfg(not(feature = "transport-stdio"))]
            unreachable!();
        }
        TransportMode::Both => {
            #[cfg(all(feature = "transport-stdio", feature = "transport-http"))]
            {
                let http_adapter = Arc::clone(&adapter);
                let port = config.http_port;
                let shutdown_signal = Arc::new(AtomicBool::new(false));
                let thread_signal = Arc::clone(&shutdown_signal);
                std::thread::spawn(move || {
                    if let Err(e) = transport::http::HttpTransport::run(http_adapter, port, Some(thread_signal)) {
                        eprintln!("[clibm-mcp] HTTP transport stopped: {}", e);
                    }
                });
                let stdio_result = transport::stdio::StdioTransport::run(adapter);
                shutdown_signal.store(true, Ordering::Relaxed);
                stdio_result
            }
            #[cfg(not(all(feature = "transport-stdio", feature = "transport-http")))]
            unreachable!();
        }
    }
}
