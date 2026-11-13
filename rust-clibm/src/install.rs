//! Machine-level installation settings shared by the CLI, plugins and installers.
//!
//! The installers (`install.ps1` / `install.sh`) record the user's choices in
//! `~/.clibm/install-config.json`. `clibm init` uses them as defaults, and
//! `clibm update` / `clibm uninstall` use them to find the installation.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Returns the user's home directory as used by CLIBM: `CLIBM_HOME`, then `HOME`,
/// then `USERPROFILE`. `CLIBM_HOME` lets tests and sandboxed installs isolate state.
pub fn clibm_home() -> Option<PathBuf> {
    env::var("CLIBM_HOME")
        .or_else(|_| env::var("HOME"))
        .or_else(|_| env::var("USERPROFILE"))
        .ok()
        .filter(|h| !h.trim().is_empty())
        .map(PathBuf::from)
}

/// `~/.clibm` (machine-level state: install config, plugin registry, PATH ledger).
pub fn global_clibm_dir() -> Option<PathBuf> {
    clibm_home().map(|h| h.join(".clibm"))
}

/// Expands a leading `~`, `~/` or `~\` using [`clibm_home`].
pub fn expand_tilde(path: &str) -> PathBuf {
    let home = clibm_home();
    match home {
        Some(h) if path == "~" => h,
        Some(h) if path.starts_with("~/") || path.starts_with("~\\") => h.join(&path[2..]),
        _ => PathBuf::from(path),
    }
}

/// Contents of `~/.clibm/install-config.json`. Every field is optional because the
/// file is written by several installer versions (and may be missing entirely).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_commit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_entry_added: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_transport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl InstallConfig {
    pub fn path() -> Option<PathBuf> {
        global_clibm_dir().map(|d| d.join("install-config.json"))
    }

    /// Loads the install config, returning defaults when it is missing or unreadable.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Option<Self> {
        let raw = fs::read_to_string(path).ok()?;
        let clean = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
        serde_json::from_str(clean).ok()
    }

    /// Writes `~/.clibm/install-config.json` (UTF-8, no BOM).
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine the home directory for ~/.clibm"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }

    /// The configured install directory with `~` expanded, if any.
    pub fn install_dir(&self) -> Option<PathBuf> {
        self.install_path
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .map(expand_tilde)
    }
}
