//! Configuration loader and CLI argument parser for clibm-mcp.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    Both,
    Stdio,
    Http,
}

impl Default for TransportMode {
    fn default() -> Self {
        TransportMode::Both
    }
}

impl TransportMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportMode::Both => "both",
            TransportMode::Stdio => "stdio",
            TransportMode::Http => "http",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub transport: TransportMode,
    pub http_port: u16,
    pub read_only: bool,
    pub project_root: PathBuf,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            transport: TransportMode::Both,
            http_port: 7879,
            read_only: false,
            project_root: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl McpConfig {
    pub fn load_from_env_and_args(args: &[String]) -> Self {
        let mut cfg = Self::default();

        // 1. Try reading ~/.clibm/mcp-config.json
        if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
            let cfg_file = PathBuf::from(home).join(".clibm").join("mcp-config.json");
            if let Ok(raw) = fs::read_to_string(&cfg_file) {
                let clean = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
                if let Ok(v) = serde_json::from_str::<Value>(clean) {
                    if let Some(t) = v.get("transport").and_then(|x| x.as_str()) {
                        match t {
                            "stdio" => cfg.transport = TransportMode::Stdio,
                            "http" => cfg.transport = TransportMode::Http,
                            _ => cfg.transport = TransportMode::Both,
                        }
                    }
                    if let Some(p) = v.get("http_port").and_then(|x| x.as_u64()) {
                        cfg.http_port = p as u16;
                    }
                    if let Some(ro) = v.get("read_only").and_then(|x| x.as_bool()) {
                        cfg.read_only = ro;
                    }
                }
            }
        }

        // 2. Override with CLIBM_PROJECT_ROOT if present
        if let Ok(root) = env::var("CLIBM_PROJECT_ROOT") {
            cfg.project_root = PathBuf::from(root);
        }

        // 3. CLI arguments override configuration
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--stdio" => cfg.transport = TransportMode::Stdio,
                "--http" => cfg.transport = TransportMode::Http,
                "--both" => cfg.transport = TransportMode::Both,
                "--read-only" => cfg.read_only = true,
                "--port" | "-p" => {
                    if i + 1 < args.len() {
                        if let Ok(p) = args[i + 1].parse::<u16>() {
                            cfg.http_port = p;
                        }
                        i += 1;
                    }
                }
                "--root" => {
                    if i + 1 < args.len() {
                        cfg.project_root = PathBuf::from(&args[i + 1]);
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        cfg
    }
}

