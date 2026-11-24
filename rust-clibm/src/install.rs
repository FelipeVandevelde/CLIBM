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
    /// Plugins chosen during installation (profiles "with-defaults" and "custom").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<String>>,
}

/// Install profiles the installers can record: core only, core + the default plugins,
/// or core + plugins the user picked from the catalog.
pub const INSTALL_PROFILES: [&str; 3] = ["pure", "with-defaults", "custom"];

/// Returns the canonical profile name for `value` (case-insensitive), if it is one.
pub fn normalize_install_profile(value: &str) -> Option<&'static str> {
    let v = value.trim().to_ascii_lowercase();
    INSTALL_PROFILES.iter().copied().find(|p| *p == v)
}

/// One installable official plugin, as listed in `plugins/catalog.json`. The installers
/// read the same file (locally or from GitHub) to show the plugins the user can pick.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub folder: String,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub description: String,
}

const CATALOG_JSON: &str = include_str!("../plugins/catalog.json");

/// The official plugin catalog built into this binary.
pub fn plugin_catalog() -> Vec<CatalogEntry> {
    serde_json::from_str(CATALOG_JSON).expect("plugins/catalog.json must be a valid JSON array")
}

/// Names of the plugins installed by the "with-defaults" profile.
pub fn default_plugins() -> Vec<String> {
    plugin_catalog()
        .into_iter()
        .filter(|p| p.default)
        .map(|p| p.name)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::official_plugin_folder;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_matches_the_official_plugin_folders() {
        let plugins_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = plugin_catalog();
        assert!(!catalog.is_empty());

        let mut names = BTreeSet::new();
        for entry in &catalog {
            assert!(names.insert(entry.name.clone()), "duplicate catalog entry {}", entry.name);
            assert_eq!(
                official_plugin_folder(&entry.name),
                Some(entry.folder.as_str()),
                "catalog entry {} does not match official_plugin_folder()",
                entry.name
            );
            assert!(
                plugins_dir.join(&entry.folder).join("Cargo.toml").is_file(),
                "catalog entry {} points at a missing folder",
                entry.name
            );
            assert!(!entry.description.trim().is_empty(), "{} has no description", entry.name);
        }

        // Every plugin crate in plugins/ is listed, except the starter template.
        let listed: BTreeSet<String> = catalog.iter().map(|e| e.folder.clone()).collect();
        for dir in fs::read_dir(&plugins_dir).unwrap().flatten() {
            let folder = dir.file_name().to_string_lossy().to_string();
            if folder == "plugin-template" || !dir.path().join("Cargo.toml").is_file() {
                continue;
            }
            assert!(listed.contains(&folder), "plugins/{} is missing from plugins/catalog.json", folder);
        }
    }

    /// The installers carry a copy of the catalog for when it cannot be downloaded.
    #[test]
    fn installer_fallback_lists_match_the_catalog() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let entries = |text: &str| -> Vec<String> {
            text.lines()
                .map(str::trim)
                .filter(|l| l.starts_with('{'))
                .map(|l| l.trim_end_matches(',').to_string())
                .collect()
        };
        let expected = entries(CATALOG_JSON);
        for script in ["install.sh", "install.ps1"] {
            let text = fs::read_to_string(root.join(script)).unwrap();
            let start = text
                .find("# >>> clibm-catalog-fallback >>>")
                .unwrap_or_else(|| panic!("{} has no catalog fallback block", script));
            let end = text
                .find("# <<< clibm-catalog-fallback <<<")
                .unwrap_or_else(|| panic!("{} has no end of the catalog fallback block", script));
            assert_eq!(
                entries(&text[start..end]),
                expected,
                "the plugin list built into {} differs from plugins/catalog.json",
                script
            );
        }
    }

    #[test]
    fn default_plugins_come_from_the_catalog() {
        assert_eq!(
            default_plugins(),
            vec!["clibm-dep-audit", "clibm-export-md", "clibm-workflow-lint"]
        );
    }

    #[test]
    fn install_profiles_are_normalized() {
        assert_eq!(normalize_install_profile(" Custom "), Some("custom"));
        assert_eq!(normalize_install_profile("with-defaults"), Some("with-defaults"));
        assert_eq!(normalize_install_profile("everything"), None);
    }
}
