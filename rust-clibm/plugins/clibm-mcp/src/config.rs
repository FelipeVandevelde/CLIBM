//! Configuration loader and CLI argument parser for clibm-mcp.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Which transports to serve. Stdio is the default (what AI clients launch); HTTP only
/// starts when asked for on the command line or in `~/.clibm/mcp-config.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    Both,
    #[default]
    Stdio,
    Http,
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
    /// Home directory holding `.clibm/plugins/registry.json`. `None` uses
    /// `CLIBM_HOME` / `HOME` / `USERPROFILE` (tests point it at a temp folder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_dir: Option<PathBuf>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            transport: TransportMode::Stdio,
            http_port: 7879,
            read_only: false,
            project_root: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            home_dir: None,
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var("CLIBM_HOME")
        .or_else(|_| env::var("HOME"))
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

impl McpConfig {
    /// Builds the configuration from `~/.clibm/mcp-config.json`, `CLIBM_PROJECT_ROOT`
    /// and the command line (in increasing priority). Unknown arguments and missing or
    /// invalid values are errors.
    pub fn load_from_env_and_args(args: &[String]) -> Result<Self, String> {
        let mut cfg = Self::default();

        // 1. ~/.clibm/mcp-config.json (written by the installer)
        if let Some(home) = home_dir() {
            let cfg_file = home.join(".clibm").join("mcp-config.json");
            if let Ok(raw) = fs::read_to_string(&cfg_file) {
                let clean = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
                if let Ok(v) = serde_json::from_str::<Value>(clean) {
                    if let Some(t) = v.get("transport").and_then(|x| x.as_str()) {
                        cfg.transport = match t {
                            "http" => TransportMode::Http,
                            "both" => TransportMode::Both,
                            _ => TransportMode::Stdio,
                        };
                    }
                    if let Some(p) = v.get("http_port").and_then(|x| x.as_u64()) {
                        if let Ok(port) = u16::try_from(p) {
                            cfg.http_port = port;
                        }
                    }
                    if let Some(ro) = v.get("read_only").and_then(|x| x.as_bool()) {
                        cfg.read_only = ro;
                    }
                }
            }
        }

        // 2. CLIBM_PROJECT_ROOT (set by `clibm plugin run`)
        if let Ok(root) = env::var("CLIBM_PROJECT_ROOT") {
            cfg.project_root = PathBuf::from(root);
        }

        // 3. Command line
        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_str();
            let mut value = || -> Result<String, String> {
                i += 1;
                args.get(i)
                    .cloned()
                    .ok_or_else(|| format!("'{}' needs a value", arg))
            };
            match arg {
                "--stdio" => cfg.transport = TransportMode::Stdio,
                "--http" => cfg.transport = TransportMode::Http,
                "--both" => cfg.transport = TransportMode::Both,
                "--read-only" => cfg.read_only = true,
                "--port" | "-p" => {
                    let v = value()?;
                    cfg.http_port = v.parse::<u16>().map_err(|_| format!("Invalid port '{}'", v))?;
                }
                "--root" => cfg.project_root = PathBuf::from(value()?),
                other => return Err(format!("Unknown argument '{}'", other)),
            }
            i += 1;
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_flags_and_rejects_unknown_ones() {
        let cfg = McpConfig::load_from_env_and_args(&args(&["--http", "--port", "9000", "--read-only"])).unwrap();
        assert_eq!(cfg.transport, TransportMode::Http);
        assert_eq!(cfg.http_port, 9000);
        assert!(cfg.read_only);

        assert!(McpConfig::load_from_env_and_args(&args(&["--version"])).is_err());
        assert!(McpConfig::load_from_env_and_args(&args(&["--bogus"])).is_err());
        assert!(McpConfig::load_from_env_and_args(&args(&["--port"])).is_err());
        assert!(McpConfig::load_from_env_and_args(&args(&["--port", "abc"])).is_err());
    }

    #[test]
    fn stdio_is_the_default_transport() {
        assert_eq!(McpConfig::default().transport, TransportMode::Stdio);
    }
}
