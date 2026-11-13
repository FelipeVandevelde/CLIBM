//! Modular CLIBM Self-Hosted MCP Server Plugin (`clibm-mcp`)
//!
//! Exposes any ongoing `.clibm` repository to AI agents and IDEs (Claude Desktop, Cursor,
//! Windsurf, custom LLM agents) via the Model Context Protocol (`2024-11-05`).
//!
//! Features:
//!   - `transport-stdio`: Minimal STDIO JSON-RPC 2.0 transport without HTTP dependencies.
//!   - `transport-http`: Local HTTP + SSE JSON-RPC 2.0 server using `tiny_http`.

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

fn print_help() {
    println!(
        "CLIBM-MCP v0.2.0 — Self-Hosted AI Model Context Protocol Server\n\n\
Documentation & Agent Setup:\n  \
https://github.com/FelipeVandevelde/CLIBM/blob/main/rust-clibm/plugins/clibm-mcp/README.md#ai-agent-mcp-configuration-json\n\n\
USAGE:\n  \
clibm plugin run mcp -- [OPTIONS]\n  \
clibm-mcp [OPTIONS]\n\n\
OPTIONS:\n  \
--stdio        Run in STDIO transport mode (JSON-RPC 2.0 over stdin/stdout)\n  \
--http         Run local HTTP JSON-RPC 2.0 server\n  \
--both         Run both STDIO (foreground) and HTTP (background thread)\n  \
--port, -p <P> Port for HTTP transport (default: 7879)\n  \
--root <PATH>  Project root directory containing .clibm (or set CLIBM_PROJECT_ROOT)\n  \
--read-only    Reject mutating tool calls (safe autonomous agent mode)\n  \
-h, --help     Print help and agent configuration guide\n\n\
AGENT MCP CONFIGURATION JSON (for Claude Desktop / Cursor / Windsurf):\n\
{{\n  \
  \"mcpServers\": {{\n    \
    \"clibm\": {{\n      \
      \"command\": \"clibm\",\n      \
      \"args\": [\"plugin\", \"run\", \"mcp\", \"--\", \"--stdio\"]\n    \
    }}\n  \
  }}\n\
}}"
    );
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

    let config = McpConfig::load_from_env_and_args(&args);

    let want_stdio_explicit = args.iter().any(|a| a == "--stdio");
    let want_http_explicit = args.iter().any(|a| a == "--http");
    let _want_both_explicit = args.iter().any(|a| a == "--both");

    // Guard against running a transport not compiled into this binary
    if want_http_explicit && !cfg!(feature = "transport-http") {
        bail!(
            "HTTP transport is not compiled into this clibm-mcp binary.\n\
             Rebuild with: cargo build --release --features transport-http"
        );
    }
    if want_stdio_explicit && !cfg!(feature = "transport-stdio") {
        bail!(
            "STDIO transport is not compiled into this clibm-mcp binary.\n\
             Rebuild with: cargo build --release --features transport-stdio"
        );
    }

    let adapter = Arc::new(McpAdapter::new(config.clone()));

    // Single mode explicit selection
    if want_http_explicit && !want_stdio_explicit {
        #[cfg(feature = "transport-http")]
        return transport::http::HttpTransport::run(adapter, config.http_port, None);

        #[cfg(not(feature = "transport-http"))]
        bail!("HTTP transport feature is disabled.");
    }

    if want_stdio_explicit && !want_http_explicit {
        #[cfg(feature = "transport-stdio")]
        return transport::stdio::StdioTransport::run(adapter);

        #[cfg(not(feature = "transport-stdio"))]
        bail!("STDIO transport feature is disabled.");
    }

    // Configuration-based or default selection
    match config.transport {
        TransportMode::Http => {
            #[cfg(feature = "transport-http")]
            return transport::http::HttpTransport::run(adapter, config.http_port, None);

            #[cfg(not(feature = "transport-http"))]
            bail!("HTTP transport feature is not compiled in.");
        }
        TransportMode::Stdio => {
            #[cfg(feature = "transport-stdio")]
            return transport::stdio::StdioTransport::run(adapter);

            #[cfg(not(feature = "transport-stdio"))]
            bail!("STDIO transport feature is not compiled in.");
        }
        TransportMode::Both => {
            // When both features are compiled into the binary:
            #[cfg(all(feature = "transport-stdio", feature = "transport-http"))]
            {
                if _want_both_explicit || !want_stdio_explicit {
                    let http_adapter = Arc::clone(&adapter);
                    let port = config.http_port;
                    let shutdown_signal = Arc::new(AtomicBool::new(false));
                    let thread_signal = Arc::clone(&shutdown_signal);

                    // Spawn HTTP server on a background daemon thread
                    std::thread::spawn(move || {
                        let _ = transport::http::HttpTransport::run(
                            http_adapter,
                            port,
                            Some(thread_signal),
                        );
                    });

                    // Run STDIO transport on the foreground main thread
                    let stdio_result = transport::stdio::StdioTransport::run(adapter);
                    shutdown_signal.store(true, Ordering::Relaxed);
                    return stdio_result;
                }
            }

            // Fallback if only one feature was compiled in for a 'both' preference:
            #[cfg(feature = "transport-stdio")]
            {
                return transport::stdio::StdioTransport::run(adapter);
            }

            #[cfg(all(not(feature = "transport-stdio"), feature = "transport-http"))]
            {
                return transport::http::HttpTransport::run(adapter, config.http_port, None);
            }
        }
    }
}
