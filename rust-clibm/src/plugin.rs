//! Extensibility & Plugin Contract System (Architectural Principle #5 — 100% Pure Core)
//!
//! `clibm_core` is 100% pure and contains zero built-in plugins.
//! All plugins (`clibm-dep-audit`, `clibm-export-md`, `clibm-workflow-lint`,
//! `clibm-mcp`, `clibm-tui`, `clibm-web`) live as standalone
//! Cargo crates in `plugins/` and communicate via `CLIBM_PLUGIN_CONTRACT_V1`.

use crate::cas::CasStore;
use crate::install::{clibm_home, expand_tilde, global_clibm_dir, InstallConfig};
use crate::models::*;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const CONTRACT_VERSION: &str = "CLIBM_PLUGIN_CONTRACT_V1";
pub const UTF8_BOM: &str = "\u{FEFF}";

const EMBEDDED_ENV_PATH_PS1: &str = include_str!("../scripts/env-path.ps1");
const EMBEDDED_ENV_PATH_SH: &str = include_str!("../scripts/env-path.sh");

fn parse_env_path_version(clean: &str) -> u32 {
    for line in clean.lines().take(5) {
        if let Some(rest) = line.trim().strip_prefix("# clibm-env-path-version:") {
            if let Ok(v) = rest.trim().parse::<u32>() {
                return v;
            }
        }
    }
    1
}

fn build_embedded_ps1_helper_block() -> String {
    let clean = EMBEDDED_ENV_PATH_PS1
        .strip_prefix(UTF8_BOM)
        .unwrap_or(EMBEDDED_ENV_PATH_PS1)
        .replace("\r\n", "\n");
    let clean = clean.trim_end_matches('\n');
    let ver = parse_env_path_version(clean);
    format!(
        "# >>> clibm-env-path (generated) >>>\n\
function Initialize-ClibmEnvPathHelper {{\n\
    try {{ Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction SilentlyContinue }} catch {{}}\n\
    $HomeDir = if ($env:CLIBM_HOME) {{ $env:CLIBM_HOME }} elseif ($HOME) {{ $HOME }} elseif ($env:USERPROFILE) {{ $env:USERPROFILE }} else {{ [Environment]::GetFolderPath('UserProfile') }}\n\
    $LibDir = Join-Path $HomeDir '.clibm\\lib'\n\
    $HelperFile = Join-Path $LibDir 'env-path.ps1'\n\
    $EmbeddedVersion = {ver}\n\
    $EmbeddedContent = @'\n\
{clean}\n\
'@\n\
    $script:ClibmEmbeddedHelperContent = $EmbeddedContent\n\
    $NeedsWrite = $true\n\
    if (Test-Path -LiteralPath $HelperFile) {{\n\
        try {{\n\
            $HeadLines = Get-Content -LiteralPath $HelperFile -TotalCount 5 -Encoding UTF8 -ErrorAction Stop\n\
            foreach ($Line in $HeadLines) {{\n\
                if ($Line -match 'clibm-env-path-version:\\s*(\\d+)') {{\n\
                    if ([int]$Matches[1] -gt $EmbeddedVersion) {{\n\
                        $NeedsWrite = $false\n\
                    }} elseif ([int]$Matches[1] -eq $EmbeddedVersion) {{\n\
                        $ExistingClean = ([System.IO.File]::ReadAllText($HelperFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF) -replace \"`r`n\", \"`n\").TrimEnd(\"`n\")\n\
                        $TargetClean = ($EmbeddedContent -replace \"`r`n\", \"`n\").TrimEnd(\"`n\")\n\
                        if ($ExistingClean -ceq $TargetClean) {{\n\
                            $NeedsWrite = $false\n\
                        }}\n\
                    }}\n\
                    break\n\
                }}\n\
            }}\n\
        }} catch {{\n\
            $NeedsWrite = $true\n\
        }}\n\
    }}\n\
    if ($NeedsWrite) {{\n\
        New-Item -ItemType Directory -Force -Path $LibDir | Out-Null\n\
        $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)\n\
        [System.IO.File]::WriteAllText($HelperFile, $EmbeddedContent, $Utf8NoBom)\n\
    }}\n\
    return $HelperFile\n\
}}\n\
try {{ . (Initialize-ClibmEnvPathHelper) }} catch {{ Invoke-Expression $script:ClibmEmbeddedHelperContent }}\n\
# <<< clibm-env-path (generated) <<<"
    )
}

fn build_embedded_sh_helper_block() -> String {
    let clean = EMBEDDED_ENV_PATH_SH
        .strip_prefix(UTF8_BOM)
        .unwrap_or(EMBEDDED_ENV_PATH_SH)
        .replace("\r\n", "\n");
    let clean = clean.trim_end_matches('\n');
    let ver = parse_env_path_version(clean);
    format!(
        "# >>> clibm-env-path (generated) >>>\n\
clibm_init_env_path_helper() {{\n\
  local home_dir=\"${{CLIBM_HOME:-${{HOME}}}}\"\n\
  local lib_dir=\"${{home_dir%/}}/.clibm/lib\"\n\
  local helper_file=\"${{lib_dir}}/env-path.sh\"\n\
  local embedded_version={ver}\n\
  local needs_write=1\n\
  local bom=$'\\xef\\xbb\\xbf'\n\
  if [ -f \"$helper_file\" ]; then\n\
    local first_three\n\
    first_three=$(head -c 3 \"$helper_file\" 2>/dev/null || true)\n\
    local existing_ver\n\
    existing_ver=$(tr -d '\\r' < \"$helper_file\" | sed \"1s/^${{bom}}//\" | sed -n '1,5s/^# clibm-env-path-version:[[:space:]]*\\([0-9][0-9]*\\).*/\\1/p' | head -n1)\n\
    if [ \"$first_three\" != \"$bom\" ] && [ -n \"$existing_ver\" ] && [ \"$existing_ver\" -gt \"$embedded_version\" ] 2>/dev/null; then\n\
      needs_write=0\n\
    fi\n\
  fi\n\
  if [ \"$needs_write\" -eq 1 ]; then\n\
    mkdir -p \"$lib_dir\"\n\
    cat <<'CLIBM_ENV_PATH_EOF' | tr -d '\\r' > \"$helper_file\"\n\
{clean}\n\
CLIBM_ENV_PATH_EOF\n\
  fi\n\
  # shellcheck disable=SC1090\n\
  . \"$helper_file\"\n\
}}\n\
clibm_init_env_path_helper\n\
# <<< clibm-env-path (generated) <<<"
    )
}

fn write_utf8_file(path: &Path, content: &str) -> Result<()> {
    let clean = content.strip_prefix(UTF8_BOM).unwrap_or(content);
    fs::write(path, clean.as_bytes())?;
    Ok(())
}

fn read_utf8_strip_bom(path: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)?;
    Ok(raw.strip_prefix(UTF8_BOM).unwrap_or(&raw).to_string())
}

/// Lifecycle events the core fires to subscribed plugins. A plugin answering
/// `ok: false` to a `pre-*` event blocks the action; other events are informational.
pub const HOOK_EVENTS: &[&str] = &[
    "pre-commit",
    "post-commit",
    "pre-move",
    "on-task-move",
    "on-task-depend",
    "on-workspace-switch",
    "on-export",
];

/// Hook names that mark a plugin as interactive (it owns the terminal or runs a server).
/// Interactive plugins are launched with the terminal attached and never receive hooks.
const INTERACTIVE_MARKERS: &[&str] = &[
    "interactive-tui",
    "http-bridge",
    "mcp-stdio",
    "mcp-http",
    "mcp-http-sse",
];

fn default_plugin_version() -> String {
    "0.1.0".to_string()
}

fn default_enabled() -> bool {
    true
}

/// An installed plugin, as stored in the global registry `~/.clibm/plugins/registry.json`.
/// Official plugins print this manifest themselves with `<binary> --manifest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default = "default_plugin_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub hooks: Vec<String>,
    /// Absolute path of the installed plugin executable.
    #[serde(default)]
    pub executable: Option<String>,
    /// Extra fields (e.g. MCP `tools`) are preserved as-is.
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

impl PluginManifest {
    /// Executable path with `~` expanded.
    pub fn executable_path(&self) -> Option<PathBuf> {
        self.executable
            .as_deref()
            .filter(|e| !e.trim().is_empty())
            .map(expand_tilde)
    }

    /// `true` when the registered executable exists on disk.
    pub fn is_installed(&self) -> bool {
        self.executable_path().map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn is_interactive(&self) -> bool {
        matches!(self.name.as_str(), "clibm-tui" | "clibm-web" | "clibm-mcp")
            || self
                .hooks
                .iter()
                .any(|h| INTERACTIVE_MARKERS.contains(&h.as_str()))
    }

    pub fn subscribes_to(&self, event: &str) -> bool {
        self.hooks.iter().any(|h| h == event)
    }
}

/// Per-project setting stored in `.clibm/plugins/registry.json`. A project can only
/// enable or disable an installed plugin; it can never point CLIBM at an executable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginOverride {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationRequest {
    pub contract: String,
    pub event: String,
    pub tree_hash: String,
    pub args: Vec<String>,
    /// Event details (task, columns, commit...). Optional, so V1 plugins keep working.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub context: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationResponse {
    pub ok: bool,
    #[serde(default)]
    pub output: String,
}

/// What `clibm plugin uninstall` did.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginUninstallReport {
    pub name: String,
    /// Binaries deleted.
    pub removed: Vec<String>,
    /// Binaries that were locked (Windows) and renamed to `*.old.<id>` instead.
    pub rotated: Vec<String>,
    /// Binaries that could neither be deleted nor renamed.
    pub failed: Vec<String>,
    pub registry_entry_removed: bool,
}

/// Reads a registry file (global manifests or project overrides). Missing or invalid
/// files read as empty; a UTF-8 BOM is tolerated.
fn read_registry_values(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let clean = raw.strip_prefix(UTF8_BOM).unwrap_or(&raw);
    serde_json::from_str::<Vec<Value>>(clean).unwrap_or_default()
}

/// Writes registry entries as a JSON array with one compact object per line (UTF-8,
/// no BOM). The shell helpers in `scripts/env-path.*` rely on this line format.
pub fn write_registry_file<T: Serialize>(path: &Path, entries: &[T]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lines = Vec::with_capacity(entries.len());
    for e in entries {
        lines.push(serde_json::to_string(e)?);
    }
    let body = if lines.is_empty() {
        "[]\n".to_string()
    } else {
        format!("[\n{}\n]\n", lines.join(",\n"))
    };
    write_utf8_file(path, &body)
        .with_context(|| format!("Failed to write plugin registry {}", path.display()))
}

pub struct PluginManager;

impl PluginManager {
    /// `~/.clibm/plugins/registry.json`: the single list of installed plugins.
    pub fn global_registry_file() -> Option<PathBuf> {
        global_clibm_dir().map(|d| d.join("plugins").join("registry.json"))
    }

    fn project_registry_file(cas: &CasStore) -> PathBuf {
        cas.plugins_dir().join("registry.json")
    }

    /// All entries of the global registry, including ones whose binary is missing.
    pub fn load_global_registry() -> Vec<PluginManifest> {
        Self::global_registry_file()
            .map(|f| read_registry_values(&f))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| serde_json::from_value::<PluginManifest>(v).ok())
            .collect()
    }

    pub fn save_global_registry(entries: &[PluginManifest]) -> Result<()> {
        let file = Self::global_registry_file()
            .ok_or_else(|| anyhow!("Cannot determine the home directory for ~/.clibm"))?;
        write_registry_file(&file, entries)
    }

    /// Project overrides. Older full manifests in the project file are read as overrides
    /// (only `name` and `enabled` are used).
    pub fn load_project_overrides(cas: &CasStore) -> Vec<PluginOverride> {
        read_registry_values(&Self::project_registry_file(cas))
            .into_iter()
            .filter_map(|v| {
                let name = v.get("name")?.as_str()?.to_string();
                let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                Some(PluginOverride { name, enabled })
            })
            .collect()
    }

    fn save_project_overrides(cas: &CasStore, overrides: &[PluginOverride]) -> Result<()> {
        write_registry_file(&Self::project_registry_file(cas), overrides)
    }

    /// Installed plugins with this project's enable/disable overrides applied, sorted by
    /// name. Entries whose executable no longer exists are removed from the global
    /// registry (with a notice on stderr) instead of being listed.
    pub fn load_registry(cas: &CasStore) -> Result<Vec<PluginManifest>> {
        let global = Self::load_global_registry();
        let (mut installed, missing): (Vec<PluginManifest>, Vec<PluginManifest>) =
            global.into_iter().partition(|p| p.is_installed());
        if !missing.is_empty() {
            for m in &missing {
                eprintln!(
                    "[clibm] Removed stale plugin entry '{}' (binary not found: {})",
                    m.name,
                    m.executable.as_deref().unwrap_or("-")
                );
            }
            let _ = Self::save_global_registry(&installed);
        }
        if cas.is_initialized() {
            let overrides = Self::load_project_overrides(cas);
            for p in installed.iter_mut() {
                if let Some(o) = overrides.iter().find(|o| o.name == p.name) {
                    p.enabled = o.enabled;
                }
            }
        }
        installed.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(installed)
    }

    /// Adds or replaces `manifest` in the global registry.
    pub fn register_installed_plugin(manifest: &PluginManifest) -> Result<()> {
        let mut entries = Self::load_global_registry();
        match entries.iter_mut().find(|p| p.name == manifest.name) {
            Some(existing) => *existing = manifest.clone(),
            None => entries.push(manifest.clone()),
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Self::save_global_registry(&entries)
    }

    /// Asks a plugin binary for its manifest (`<exe> --manifest --executable <exe>`).
    /// Returns `None` for plugins that do not support it.
    pub fn query_manifest(exe: &Path, hooks: Option<&str>) -> Option<PluginManifest> {
        let mut args = vec![
            "--manifest".to_string(),
            "--executable".to_string(),
            exe.to_string_lossy().to_string(),
        ];
        if let Some(h) = hooks.filter(|h| !h.trim().is_empty()) {
            args.push("--hooks".to_string());
            args.push(h.to_string());
        }
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let (ok, out) =
            run_plugin_process(exe, &args, &cwd, None, Some(Duration::from_secs(10)), false).ok()?;
        if !ok {
            return None;
        }
        let line = out.lines().map(str::trim).find(|l| l.starts_with('{'))?;
        let mut manifest: PluginManifest = serde_json::from_str(line).ok()?;
        manifest.executable = Some(exe.to_string_lossy().replace('\\', "/"));
        Some(manifest)
    }

    /// Resolves short plugin aliases to their standalone crate binary names in `plugins/`.
    pub fn normalize_plugin_name(name: &str) -> String {
        match name.trim() {
            "tui" => "clibm-tui".to_string(),
            "web" => "clibm-web".to_string(),
            "mcp" => "clibm-mcp".to_string(),
            // Legacy alias: lets `clibm plugin uninstall docs` remove copies of the retired clibm-docs plugin.
            "docs" => "clibm-docs".to_string(),
            "dep-audit" => "clibm-dep-audit".to_string(),
            "export-md" => "clibm-export-md".to_string(),
            "workflow-lint" => "clibm-workflow-lint".to_string(),
            "template" | "plugin-template" => "clibm-plugin-template".to_string(),
            other => other.to_string(),
        }
    }

    pub fn binary_file_name(bin_name: &str) -> String {
        if cfg!(windows) && !bin_name.to_ascii_lowercase().ends_with(".exe") {
            format!("{}.exe", bin_name)
        } else {
            bin_name.to_string()
        }
    }

    /// Directories where CLIBM binaries are normally installed, most specific first.
    fn candidate_bin_dirs(cas: Option<&CasStore>) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = Vec::new();
        let mut push = |d: PathBuf| {
            if !dirs.contains(&d) {
                dirs.push(d);
            }
        };
        if let Some(d) = InstallConfig::load().install_dir() {
            push(d);
        }
        if let Some(cas) = cas {
            if let Ok(cfg) = cas.load_config() {
                if !cfg.install_path.trim().is_empty() {
                    push(expand_tilde(&cfg.install_path));
                }
            }
        }
        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                push(dir.to_path_buf());
            }
        }
        if let Some(h) = clibm_home() {
            push(h.join(".clibm-bin"));
            push(h.join(".local").join("bin"));
            push(h.join(".cargo").join("bin"));
        }
        dirs
    }

    /// Checks whether a binary exists in the usual install folders or on `PATH`.
    pub fn find_installed_binary(bin_name: &str) -> Option<PathBuf> {
        let exe_name = Self::binary_file_name(bin_name);

        let explicit = expand_tilde(&exe_name);
        if explicit.is_absolute() {
            return explicit.is_file().then_some(explicit);
        }

        for dir in Self::candidate_bin_dirs(None) {
            let candidate = dir.join(&exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        if let Ok(path_var) = env::var("PATH") {
            for dir in env::split_paths(&path_var) {
                let full = dir.join(&exe_name);
                if full.is_file() {
                    return Some(full);
                }
            }
        }

        None
    }

    /// Checks whether `target_dir` is currently present on the active `PATH` environment variable.
    pub fn is_dir_on_env_path(target_dir: &Path) -> bool {
        let Ok(path_var) = env::var("PATH") else {
            return false;
        };
        let norm_target = fs::canonicalize(target_dir).unwrap_or_else(|_| target_dir.to_path_buf());
        for entry in env::split_paths(&path_var) {
            if entry.as_os_str().is_empty() {
                continue;
            }
            let norm_entry = fs::canonicalize(&entry).unwrap_or_else(|_| entry.clone());
            if cfg!(windows) {
                let a = norm_target
                    .to_string_lossy()
                    .trim_end_matches(['\\', '/'])
                    .to_lowercase();
                let b = norm_entry
                    .to_string_lossy()
                    .trim_end_matches(['\\', '/'])
                    .to_lowercase();
                if a == b {
                    return true;
                }
            } else if norm_target == norm_entry {
                return true;
            }
        }
        false
    }

    /// Reads `name` and `description` from a crate's `Cargo.toml`.
    fn read_crate_identity(cargo_toml: &Path) -> Result<(String, String)> {
        let toml_text = read_utf8_strip_bom(cargo_toml)?;
        let field = |key: &str| {
            toml_text.lines().find_map(|line| {
                line.trim()
                    .strip_prefix(key)
                    .and_then(|r| r.trim().strip_prefix('='))
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
        };
        let name = field("name")
            .ok_or_else(|| anyhow!("Could not parse package name from {}", cargo_toml.display()))?;
        let description =
            field("description").unwrap_or_else(|| format!("Installed CLIBM plugin ({})", name));
        Ok((name, description))
    }

    /// Builds and installs a plugin, then registers it globally from its own manifest.
    ///
    /// `target` is a folder containing the plugin's `Cargo.toml`, or the name of an
    /// official plugin (`mcp`, `clibm-tui`, ...). Official plugins are built from a local
    /// clone when the current directory is one, otherwise from the repository recorded
    /// by the installer (default: the official GitHub repository).
    pub fn install_plugin_from_dir(
        cas: &CasStore,
        target: &Path,
        mcp_transport: Option<&str>,
    ) -> Result<PluginManifest> {
        use crate::update::{cargo_install, mcp_transport_args, official_plugin_folder, CrateLocation, UpdateSource};

        let (name, description, location) = if target.join("Cargo.toml").is_file() {
            let (name, description) = Self::read_crate_identity(&target.join("Cargo.toml"))?;
            (name, description, CrateLocation::Dir(target.to_path_buf()))
        } else {
            let leaf = target
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let name = Self::normalize_plugin_name(leaf);
            let folder = official_plugin_folder(&name).ok_or_else(|| {
                anyhow!(
                    "No Cargo.toml found in '{}'. Pass a plugin folder or an official plugin name (mcp, tui, web, dep-audit, export-md, workflow-lint, template).",
                    target.display()
                )
            })?;
            let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let local = [
                cwd.join("plugins").join(folder),
                cwd.join("rust-clibm").join("plugins").join(folder),
            ]
            .into_iter()
            .find(|d| d.join("Cargo.toml").is_file());
            match local {
                Some(dir) => {
                    let (_, description) = Self::read_crate_identity(&dir.join("Cargo.toml"))?;
                    (name, description, CrateLocation::Dir(dir))
                }
                None => {
                    let source = UpdateSource::resolve(None, None, None, cas.load_config().ok().as_ref(), &InstallConfig::load())?;
                    let location = match source {
                        UpdateSource::Git { repo, branch } => CrateLocation::Git { url: repo.clone_url, branch },
                        UpdateSource::Path(ws) => CrateLocation::Dir(ws.join("plugins").join(folder)),
                    };
                    (name.clone(), format!("Official CLIBM plugin {}", name), location)
                }
            }
        };

        let extra = if name == "clibm-mcp" {
            mcp_transport_args(mcp_transport.unwrap_or("stdio"))
        } else {
            Vec::new()
        };
        let built = cargo_install(&location, &name, &extra)?;

        let install_dir = InstallConfig::load().install_dir().unwrap_or_else(|| {
            let cfg = cas.load_config().unwrap_or_default();
            let home = clibm_home().unwrap_or_else(|| PathBuf::from("."));
            if !cfg.install_path.is_empty() && cfg.install_path != "~/.local/bin" {
                expand_tilde(&cfg.install_path)
            } else if cfg!(windows) {
                home.join(".clibm-bin")
            } else {
                home.join(".local").join("bin")
            }
        });

        fs::create_dir_all(&install_dir)?;
        let bin_filename = Self::binary_file_name(&name);
        let installed_bin = install_dir.join(&bin_filename);
        replace_binary(&built.path, &installed_bin)?;

        if !Self::is_dir_on_env_path(&install_dir) {
            println!(
                "==> Note: '{}' is not currently on your user PATH. Run the plugin's install script (`install.ps1` / `install.sh`) or add '{}' to your user PATH to invoke '{}' directly.",
                install_dir.display(),
                install_dir.display(),
                name
            );
        }

        if name == "clibm-mcp" {
            if let Ok(mut updated_cfg) = cas.load_config() {
                updated_cfg.mcp_transport = mcp_transport.unwrap_or("stdio").to_string();
                let _ = cas.save_config(&updated_cfg);
            }
        }

        // The plugin describes itself (hooks, MCP tools); fall back to a manual-run entry
        // for third-party plugins that do not implement `--manifest`.
        let manifest = Self::query_manifest(&installed_bin, None).unwrap_or_else(|| PluginManifest {
            name: name.clone(),
            version: "0.1.0".to_string(),
            description,
            enabled: true,
            hooks: vec!["manual-run".to_string()],
            executable: Some(installed_bin.to_string_lossy().replace('\\', "/")),
            extra: BTreeMap::new(),
        });
        Self::register_installed_plugin(&manifest)?;

        Ok(manifest)
    }

    /// Scaffolds a new standalone plugin crate in `plugins/<crate_name>` using the official
    /// CLIBM plugin template (`Cargo.toml`, `src/main.rs`, `README.md`, `install.sh`, `install.ps1`, `uninstall.sh`, `uninstall.ps1`)
    /// with UTF-8 encoding. The plugin is registered when its installer runs.
    pub fn scaffold_plugin_template(
        cas: &CasStore,
        raw_name: &str,
        description: &str,
        hooks: Vec<String>,
    ) -> Result<(PathBuf, PluginManifest)> {
        let slug = raw_name
            .trim()
            .to_lowercase()
            .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-");
        if slug.is_empty() {
            bail!("Plugin name cannot be empty");
        }
        let crate_name = if slug.starts_with("clibm-") {
            slug
        } else {
            format!("clibm-{}", slug)
        };

        // Escaped for use inside TOML / Rust string literals.
        let description = description.replace('\\', "\\\\").replace('"', "\\\"");
        let hooks: Vec<String> = hooks
            .into_iter()
            .map(|h| h.trim().to_lowercase())
            .filter(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
            .collect();
        let hooks_rust = hooks
            .iter()
            .map(|h| format!("\"{}\"", h))
            .collect::<Vec<_>>()
            .join(", ");

        let plugin_dir = cas.workspace_root().join("plugins").join(&crate_name);
        let src_dir = plugin_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        let cargo_toml = format!(
            r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"
description = "{description}"

[[bin]]
name = "{crate_name}"
path = "src/main.rs"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"
"#
        );
        write_utf8_file(&plugin_dir.join("Cargo.toml"), &cargo_toml)?;

        let main_rs = format!(
            r#"//! Standalone CLIBM Plugin: `{crate_name}`
//! Description: {description}
//!
//! - `{crate_name} --manifest` prints this plugin's registry entry (used by installers).
//! - When CLIBM runs it (`clibm plugin run` or a lifecycle hook), a CLIBM_PLUGIN_CONTRACT_V1
//!   JSON request arrives on stdin and a `{{"ok": bool, "output": "..."}}` reply is expected
//!   on stdout. Answering `ok: false` to a `pre-*` event blocks that action.

use anyhow::Result;
use serde::{{Deserialize, Serialize}};
use std::env;
use std::io::{{self, IsTerminal, Read}};
use std::path::PathBuf;

const NAME: &str = "{crate_name}";
const DESCRIPTION: &str = "{description}";
/// Lifecycle events this plugin subscribes to (pre-commit, post-commit, pre-move,
/// on-task-move, on-task-depend, on-workspace-switch, on-export).
const DEFAULT_HOOKS: &[&str] = &[{hooks_rust}];

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Request {{
    contract: String,
    event: String,
    tree_hash: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    context: serde_json::Value,
}}

#[derive(Debug, Serialize)]
struct Response {{
    ok: bool,
    output: String,
}}

fn main() -> Result<()> {{
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--manifest") {{
        println!("{{}}", manifest_json(&args));
        return Ok(());
    }}

    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut input = String::new();
    if !io::stdin().is_terminal() {{
        let _ = io::stdin().read_to_string(&mut input);
    }}
    match serde_json::from_str::<Request>(input.trim()) {{
        Ok(req) => {{
            let output = format!(
                "[{{}}] event={{}} tree={{}} context={{}} project={{}}",
                NAME,
                req.event,
                &req.tree_hash[..req.tree_hash.len().min(12)],
                req.context,
                project_root.display()
            );
            println!("{{}}", serde_json::to_string(&Response {{ ok: true, output }})?);
        }}
        Err(_) => println!("[{{}}] Running in CLIBM project: {{}}", NAME, project_root.display()),
    }}
    Ok(())
}}

/// Registry entry printed for `--manifest [--executable <path>] [--hooks a,b]`.
fn manifest_json(args: &[String]) -> String {{
    let value_after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let hooks: Vec<String> = match value_after("--hooks") {{
        Some(csv) => csv.split(',').map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).collect(),
        None => DEFAULT_HOOKS.iter().map(|h| h.to_string()).collect(),
    }};
    serde_json::json!({{
        "name": NAME,
        "version": env!("CARGO_PKG_VERSION"),
        "description": DESCRIPTION,
        "enabled": true,
        "hooks": hooks,
        "executable": value_after("--executable"),
    }})
    .to_string()
}}
"#
        );
        write_utf8_file(&src_dir.join("main.rs"), &main_rs)?;

        let readme = format!(
            "# {crate_name}\n\n{description}\n\n- **Encoding:** UTF-8\n\n## Quick Install\n\n### Linux, macOS, WSL2\n```bash\ncurl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/{crate_name}/install.sh | bash\n```\n\n### Windows (native, PowerShell)\n```powershell\niex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/{crate_name}/install.ps1)\n```\n\n## Quick Uninstall\n\n### Linux, macOS, WSL2\n```bash\ncurl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/{crate_name}/uninstall.sh | bash\n```\n\n### Windows (native, PowerShell)\n```powershell\niex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/{crate_name}/uninstall.ps1)\n```\n\n## Local Build & Install (From This Folder)\n\n```bash\ncd plugins/{crate_name}\n./install.sh\n# Or on Windows PowerShell:\n.\\install.ps1\n```\n\n## Run in Any Ongoing CLIBM Project\n\n```bash\nclibm plugin run {crate_name}\n```\n\n## Local Uninstall\n\n```bash\n./uninstall.sh\n# Or on Windows PowerShell:\n.\\uninstall.ps1\n```\n"
        );
        write_utf8_file(&plugin_dir.join("README.md"), &readme)?;

        let ps1_helper_block = build_embedded_ps1_helper_block();
        let sh_helper_block = build_embedded_sh_helper_block();
        let default_hooks: Vec<String> = if hooks.is_empty() {
            vec!["pre-commit".to_string(), "on-task-move".to_string()]
        } else {
            hooks.clone()
        };
        let default_hooks_csv = default_hooks.join(",");
        let default_hooks_ps1 = default_hooks
            .iter()
            .map(|h| format!("\"{}\"", h))
            .collect::<Vec<_>>()
            .join(", ");

        let install_sh = format!(
            "#!/usr/bin/env bash\n\
# Encoding: UTF-8 (without BOM)\n\
set -euo pipefail\n\
CALLER_DIR=\"$(pwd)\"\n\
if [ -n \"${{BASH_SOURCE[0]:-}}\" ] && [ -f \"${{BASH_SOURCE[0]}}\" ]; then\n\
  SCRIPT_DIR=\"$(cd \"$(dirname \"${{BASH_SOURCE[0]}}\")\" && pwd)\"\n\
  cd \"$SCRIPT_DIR\"\n\
else\n\
  SCRIPT_DIR=\"$CALLER_DIR\"\n\
fi\n\
HOOK_MODE=\"\"\n\
SCOPE=\"\"\n\
INSTALL_DIR=\"${{INSTALL_DIR:-}}\"\n\
RAW_BASE_URL=\"\"\n\
NO_MODIFY_PATH=false\n\
NON_INTERACTIVE=false\n\
while [[ $# -gt 0 ]]; do\n\
  case \"$1\" in\n\
    --hooks|-h) HOOK_MODE=\"$2\"; shift 2 ;;\n\
    --scope|-s) SCOPE=\"$2\"; shift 2 ;;\n\
    --install-dir|--path) INSTALL_DIR=\"$2\"; shift 2 ;;\n\
    --raw-base) RAW_BASE_URL=\"$2\"; shift 2 ;;\n\
    --no-modify-path) NO_MODIFY_PATH=true; shift ;;\n\
    -y|--yes) NON_INTERACTIVE=true; shift ;;\n\
    *) shift ;;\n\
  esac\n\
done\n\
{sh_helper_block}\n\
if [ -z \"$HOOK_MODE\" ]; then\n\
  if [ \"$NON_INTERACTIVE\" = true ]; then\n\
    HOOK_MODE=\"default-hooks\"\n\
  else\n\
    echo \"[1/3] Select Hook Subscription Mode for {crate_name}:\"\n\
    echo \"  1) Default Hooks ({default_hooks_csv}) [Default]\"\n\
    echo \"  2) Manual Only\"\n\
    echo \"  3) All Hooks\"\n\
    clibm_prompt \"Select option [1-3, default: 1]: \" c\n\
    case \"${{c:-1}}\" in 2) HOOK_MODE=\"manual-only\" ;; 3) HOOK_MODE=\"all-hooks\" ;; *) HOOK_MODE=\"default-hooks\" ;; esac\n\
  fi\n\
fi\n\
if [ -z \"$SCOPE\" ]; then\n\
  if [ \"$NON_INTERACTIVE\" = true ]; then\n\
    SCOPE=\"global\"\n\
  else\n\
    echo \"[2/3] Select Plugin Registration Scope:\"\n\
    echo \"  1) global [Default]\"\n\
    echo \"  2) project-only\"\n\
    clibm_prompt \"Select option [1-2, default: 1]: \" s\n\
    case \"${{s:-1}}\" in 2|project|project-only) SCOPE=\"project-only\" ;; *) SCOPE=\"global\" ;; esac\n\
  fi\n\
fi\n\
if [ -z \"$INSTALL_DIR\" ]; then\n\
  if [ \"$NON_INTERACTIVE\" = true ]; then\n\
    INSTALL_DIR=\"$HOME/.local/bin\"\n\
  else\n\
    clibm_prompt \"[3/3] Specify install directory [default: $HOME/.local/bin]: \" d\n\
    INSTALL_DIR=\"${{d:-$HOME/.local/bin}}\"\n\
  fi\n\
fi\n\
INSTALL_DIR=$(clibm_normalize_dir \"$INSTALL_DIR\")\n\
if ! clibm_core_installed \"$INSTALL_DIR\"; then\n\
  echo \"==> Warning: CLIBM core installation not detected. {crate_name} depends on CLIBM core at runtime; proceeding with standalone plugin installation...\"\n\
fi\n\
mkdir -p \"$INSTALL_DIR\"\n\
clibm_build_and_stage \"{crate_name}\" \"plugins/{crate_name}\" \"$INSTALL_DIR/{crate_name}\" \"$SCRIPT_DIR\" \"$RAW_BASE_URL\" \"Cargo.toml\" \"src/main.rs\"\n\
case \"$HOOK_MODE\" in\n\
  manual-only)   RESOLVED_HOOKS=\"manual-run\" ;;\n\
  all-hooks)     RESOLVED_HOOKS=\"pre-commit,post-commit,on-task-move,on-workspace-switch,on-export\" ;;\n\
  default-hooks) RESOLVED_HOOKS=\"{default_hooks_csv}\" ;;\n\
  *)             RESOLVED_HOOKS=\"$HOOK_MODE\" ;;\n\
esac\n\
clibm_register_global_plugin \"{crate_name}\" \"{description}\" \"$RESOLVED_HOOKS\" \"$INSTALL_DIR/{crate_name}\" \"0.1.0\" \"$SCOPE\" \"$CALLER_DIR\" \"$INSTALL_DIR\"\n\
if [ \"$NO_MODIFY_PATH\" = false ]; then\n\
  clibm_path_add \"{crate_name}\" \"$INSTALL_DIR\"\n\
  echo \"==> User PATH ($INSTALL_DIR): $CLIBM_LAST_PATH_STATUS\"\n\
else\n\
  echo \"==> User PATH: skipped (--no-modify-path)\"\n\
fi\n\
echo \"Installed {crate_name} to $INSTALL_DIR/{crate_name} (hooks: $HOOK_MODE, scope: $SCOPE)\"\n"
        );
        write_utf8_file(&plugin_dir.join("install.sh"), &install_sh)?;

        let install_ps1 = format!(
            "# Encoding: UTF-8 (without BOM)\n\
$Hooks = \"\"\n\
$Scope = \"\"\n\
$InstallDir = \"\"\n\
$RawBaseUrl = \"\"\n\
$NoModifyPath = $false\n\
$Yes = $false\n\
for ($i = 0; $i -lt $args.Count; $i++) {{\n\
  switch -Regex ($args[$i]) {{\n\
    '^-+(?i)Hooks$'        {{ $Hooks = $args[++$i] }}\n\
    '^-+(?i)Scope$'        {{ $Scope = $args[++$i] }}\n\
    '^-+(?i)InstallDir$'   {{ $InstallDir = $args[++$i] }}\n\
    '^-+(?i)RawBaseUrl$'   {{ $RawBaseUrl = $args[++$i] }}\n\
    '^-+(?i)NoModifyPath$' {{ $NoModifyPath = $true }}\n\
    '^-+(?i)Yes$'          {{ $Yes = $true }}\n\
  }}\n\
}}\n\
$ErrorActionPreference = \"Stop\"\n\
$CallerDir = (Get-Location).ProviderPath\n\
$ScriptDir = if ($PSScriptRoot) {{ $PSScriptRoot }} else {{ $CallerDir }}\n\
if ($PSScriptRoot) {{ Set-Location -Path $PSScriptRoot }}\n\
{ps1_helper_block}\n\
if (-not $Hooks) {{\n\
  if ($Yes) {{ $Hooks = \"default-hooks\" }} else {{\n\
    $c = Read-Host \"[1/3] Select Hook Mode for {crate_name} (1=default-hooks [Default], 2=manual-only, 3=all-hooks)\"\n\
    switch ($c) {{ \"2\" {{ $Hooks = \"manual-only\" }} \"3\" {{ $Hooks = \"all-hooks\" }} default {{ $Hooks = \"default-hooks\" }} }}\n\
  }}\n\
}}\n\
if (-not $Scope) {{\n\
  if ($Yes) {{ $Scope = \"global\" }} else {{\n\
    $s = Read-Host \"[2/3] Select Registration Scope (1=global [Default], 2=project-only)\"\n\
    if ($s -eq \"2\") {{ $Scope = \"project-only\" }} else {{ $Scope = \"global\" }}\n\
  }}\n\
}}\n\
$HomeDir = Get-ClibmHomeDirectory\n\
$DefaultBin = Join-Path $HomeDir \".clibm-bin\"\n\
if (-not $InstallDir) {{\n\
  if ($Yes) {{ $InstallDir = $DefaultBin }} else {{\n\
    $d = Read-Host \"[3/3] Specify install directory [default: $DefaultBin]\"\n\
    if ([string]::IsNullOrWhiteSpace($d)) {{ $InstallDir = $DefaultBin }} else {{ $InstallDir = $d }}\n\
  }}\n\
}}\n\
$InstallDir = Resolve-ClibmNormalizedDir $InstallDir\n\
if (-not (Test-ClibmCoreInstalled -ExtraDir $InstallDir)) {{\n\
  Write-Host \"==> Warning: CLIBM core installation not detected. {crate_name} depends on CLIBM core at runtime; proceeding with standalone plugin installation...\" -ForegroundColor Yellow\n\
}}\n\
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null\n\
$InstalledExe = Join-Path $InstallDir \"{crate_name}.exe\"\n\
Invoke-ClibmBuildAndStage -CrateName \"{crate_name}\" -RemoteSubPath \"plugins/{crate_name}\" -SourceFiles @(\"Cargo.toml\", \"src/main.rs\") -DestinationBinary $InstalledExe -LocalCandidateDir $ScriptDir -RawBaseUrl $RawBaseUrl | Out-Null\n\
$ResolvedHooks = switch ($Hooks) {{\n\
  \"manual-only\"   {{ @(\"manual-run\") }}\n\
  \"all-hooks\"     {{ @(\"pre-commit\", \"post-commit\", \"on-task-move\", \"on-workspace-switch\", \"on-export\") }}\n\
  \"default-hooks\" {{ @({default_hooks_ps1}) }}\n\
  default         {{ @($Hooks -split ',' | Where-Object {{ $_ }}) }}\n\
}}\n\
Register-ClibmGlobalPlugin -Name \"{crate_name}\" -Description \"{description}\" -Hooks $ResolvedHooks -Executable $InstalledExe -Scope $Scope -ProjectDir $CallerDir -InstallDir $InstallDir\n\
if (-not $NoModifyPath) {{\n\
  $PathRes = Add-ClibmPathEntry -Owner \"{crate_name}\" -Dir $InstallDir\n\
  Write-Host \"==> User PATH ($InstallDir): $($PathRes.Status)\"\n\
}} else {{\n\
  Write-Host \"==> User PATH: skipped (-NoModifyPath)\"\n\
}}\n\
Write-Host \"Installed {crate_name}.exe to $InstallDir (hooks: $Hooks, scope: $Scope)\"\n"
        );
        write_utf8_file(&plugin_dir.join("install.ps1"), &install_ps1)?;

        let uninstall_sh = format!(
            "#!/usr/bin/env bash
# Encoding: UTF-8 (without BOM)
set -euo pipefail
{sh_helper_block}
clibm_uninstall_plugin \"{crate_name}\"
echo \"Uninstalled {crate_name}\"
"
        );
        write_utf8_file(&plugin_dir.join("uninstall.sh"), &uninstall_sh)?;

        let uninstall_ps1 = format!(
            "# Encoding: UTF-8 (without BOM)
$ErrorActionPreference = \"Stop\"
{ps1_helper_block}
Uninstall-ClibmPlugin -Name \"{crate_name}\"
Write-Host \"Uninstalled {crate_name}.exe\"
"
        );
        write_utf8_file(&plugin_dir.join("uninstall.ps1"), &uninstall_ps1)?;

        // Not registered yet: the binary does not exist until the plugin is built and
        // installed (its installer registers it from `--manifest`).
        let manifest = PluginManifest {
            name: crate_name.clone(),
            version: "0.1.0".to_string(),
            description,
            enabled: true,
            hooks: if hooks.is_empty() {
                vec!["manual-run".to_string()]
            } else {
                hooks
            },
            executable: None,
            extra: BTreeMap::new(),
        };

        Ok((plugin_dir, manifest))
    }

    /// Enables or disables an installed plugin. Inside a repository this is a project
    /// override; outside one it changes the global default.
    pub fn set_plugin_enabled(cas: &CasStore, name: &str, enabled: bool) -> Result<()> {
        let canonical_name = Self::normalize_plugin_name(name);
        let installed = Self::load_registry(cas)?;
        if !installed.iter().any(|p| p.name == canonical_name) {
            bail!(
                "Plugin '{}' is not installed. Install it with `clibm plugin install {}`.",
                canonical_name,
                canonical_name
            );
        }
        if cas.is_initialized() {
            let mut overrides = Self::load_project_overrides(cas);
            match overrides.iter_mut().find(|o| o.name == canonical_name) {
                Some(o) => o.enabled = enabled,
                None => overrides.push(PluginOverride {
                    name: canonical_name,
                    enabled,
                }),
            }
            Self::save_project_overrides(cas, &overrides)
        } else {
            let mut global = Self::load_global_registry();
            if let Some(p) = global.iter_mut().find(|p| p.name == canonical_name) {
                p.enabled = enabled;
            }
            Self::save_global_registry(&global)
        }
    }

    /// Uninstalls a plugin: stops its running processes (Windows), deletes its binary
    /// from the usual install folders (renaming it when Windows keeps it locked), and
    /// removes it from the global registry and from this project's overrides. The user
    /// `PATH` is left alone because the install folder is shared with the core.
    pub fn uninstall_plugin(cas: &CasStore, name: &str) -> Result<PluginUninstallReport> {
        let canonical = Self::normalize_plugin_name(name);
        if canonical == "clibm" || canonical.trim().is_empty() {
            bail!("Use `clibm uninstall` (or the core uninstall script) to remove CLIBM itself.");
        }
        let exe_name = Self::binary_file_name(&canonical);
        let mut report = PluginUninstallReport {
            name: canonical.clone(),
            ..Default::default()
        };

        let global = Self::load_global_registry();
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Some(dir) = global
            .iter()
            .find(|p| p.name == canonical)
            .and_then(|p| p.executable_path())
            .and_then(|p| p.parent().map(Path::to_path_buf))
        {
            dirs.push(dir);
        }
        for d in Self::candidate_bin_dirs(Some(cas)) {
            if !dirs.contains(&d) {
                dirs.push(d);
            }
        }
        let targets: Vec<PathBuf> = dirs
            .iter()
            .map(|d| d.join(&exe_name))
            .filter(|p| p.is_file())
            .collect();

        if cfg!(windows) && !targets.is_empty() {
            // A running plugin (e.g. an MCP server started by an AI client) keeps its .exe locked.
            let _ = Command::new("taskkill")
                .args(["/F", "/IM", &exe_name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            std::thread::sleep(Duration::from_millis(300));
        }

        for target in targets {
            match fs::remove_file(&target) {
                Ok(()) => report.removed.push(target.display().to_string()),
                Err(_) => {
                    let rotated = target.with_file_name(format!("{}.old.{}", exe_name, unique_suffix()));
                    match fs::rename(&target, &rotated) {
                        Ok(()) => report.rotated.push(rotated.display().to_string()),
                        Err(_) => report.failed.push(target.display().to_string()),
                    }
                }
            }
        }
        for dir in &dirs {
            cleanup_rotated_binaries(dir, &exe_name);
        }

        let remaining: Vec<PluginManifest> =
            global.iter().filter(|p| p.name != canonical).cloned().collect();
        if remaining.len() != global.len() {
            Self::save_global_registry(&remaining)?;
            report.registry_entry_removed = true;
        }
        if cas.is_initialized() {
            let overrides = Self::load_project_overrides(cas);
            let kept: Vec<PluginOverride> =
                overrides.iter().filter(|o| o.name != canonical).cloned().collect();
            if kept.len() != overrides.len() {
                Self::save_project_overrides(cas, &kept)?;
                report.registry_entry_removed = true;
            }
        }
        Ok(report)
    }

    /// Executes an installed plugin binary against the project in `cas`. Interactive
    /// plugins (TUI, web, MCP) get the terminal; others use the JSON stdin/stdout contract.
    /// Only executables from the global registry are ever run.
    pub fn execute_plugin(
        cas: &CasStore,
        name: &str,
        tree_hash: &str,
        _tree: &TreeSnapshotPayload,
        args: &[String],
    ) -> Result<PluginInvocationResponse> {
        let canonical_name = Self::normalize_plugin_name(name);
        let plugins = Self::load_registry(cas)?;
        let manifest = plugins
            .iter()
            .find(|p| p.name == canonical_name)
            .ok_or_else(|| {
                anyhow!(
                    "Plugin '{}' is not installed. Install it with `clibm plugin install {}` (or its install script).",
                    canonical_name,
                    canonical_name
                )
            })?;

        if !manifest.enabled {
            bail!(
                "Plugin '{}' is currently disabled. Run `clibm plugin enable {}` first.",
                canonical_name,
                canonical_name
            );
        }

        let exe = manifest
            .executable_path()
            .ok_or_else(|| anyhow!("Plugin '{}' has no executable registered", canonical_name))?;
        let project_root = cas.workspace_root();

        if manifest.is_interactive() {
            let status = Command::new(&exe)
                .args(args)
                .current_dir(&project_root)
                .env("CLIBM_PROJECT_ROOT", &project_root)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .with_context(|| {
                    format!(
                        "Failed to launch installed plugin binary '{}' in ongoing project {}",
                        exe.display(),
                        project_root.display()
                    )
                })?;

            return Ok(PluginInvocationResponse {
                ok: status.success(),
                output: String::new(),
            });
        }

        let req = PluginInvocationRequest {
            contract: CONTRACT_VERSION.to_string(),
            event: "manual-run".to_string(),
            tree_hash: tree_hash.to_string(),
            args: args.to_vec(),
            context: Value::Null,
        };
        let payload = serde_json::to_vec(&req)?;
        let (ok, out) = run_plugin_process(&exe, args, &project_root, Some(&payload), None, false)?;
        Ok(parse_plugin_response(&out, ok))
    }

    /// `true` when hooks are switched off for this process (`CLIBM_NO_HOOKS=1`, or we are
    /// running inside another plugin's hook, signalled by `CLIBM_IN_HOOK`).
    pub fn hooks_disabled_by_env() -> bool {
        let truthy = |k: &str| {
            env::var(k)
                .map(|v| !v.trim().is_empty() && v.trim() != "0" && !v.eq_ignore_ascii_case("false"))
                .unwrap_or(false)
        };
        truthy("CLIBM_NO_HOOKS") || truthy("CLIBM_IN_HOOK")
    }

    /// Fires `event` to every enabled, installed, non-interactive plugin subscribed to it.
    /// A `pre-*` hook that answers `ok: false`, fails or times out blocks the action
    /// (returns an error). Problems in other hooks are only reported on stderr.
    pub fn fire_hook(
        cas: &CasStore,
        event: &str,
        tree_hash: &str,
        context: Value,
        timeout: Duration,
    ) -> Result<()> {
        if Self::hooks_disabled_by_env() {
            return Ok(());
        }
        let blocking = event.starts_with("pre-");
        let plugins = Self::load_registry(cas)?;
        let project_root = cas.workspace_root();
        for plugin in plugins
            .iter()
            .filter(|p| p.enabled && p.subscribes_to(event) && !p.is_interactive())
        {
            let Some(exe) = plugin.executable_path() else {
                continue;
            };
            let req = PluginInvocationRequest {
                contract: CONTRACT_VERSION.to_string(),
                event: event.to_string(),
                tree_hash: tree_hash.to_string(),
                args: Vec::new(),
                context: context.clone(),
            };
            let payload = serde_json::to_vec(&req)?;
            let result = run_plugin_process(&exe, &[], &project_root, Some(&payload), Some(timeout), true)
                .map(|(ok, out)| parse_plugin_response(&out, ok));
            match result {
                Ok(resp) if resp.ok => {}
                Ok(resp) => {
                    if blocking {
                        bail!(
                            "Blocked by plugin '{}' ({} hook): {}\n(Disable it with `clibm plugin disable {}` or skip hooks once with --no-hooks.)",
                            plugin.name,
                            event,
                            resp.output.trim(),
                            plugin.name
                        );
                    }
                    eprintln!(
                        "[clibm] Plugin '{}' reported a problem on {}: {}",
                        plugin.name,
                        event,
                        resp.output.trim()
                    );
                }
                Err(e) => {
                    if blocking {
                        bail!(
                            "Plugin '{}' failed on the {} hook: {}\n(Skip hooks once with --no-hooks.)",
                            plugin.name,
                            event,
                            e
                        );
                    }
                    eprintln!("[clibm] Plugin '{}' failed on {}: {}", plugin.name, event, e);
                }
            }
        }
        Ok(())
    }
}

/// Interprets a plugin's stdout: the whole output (or its last JSON line) as a
/// `PluginInvocationResponse`, otherwise the raw text with the process exit status.
fn parse_plugin_response(stdout: &str, process_ok: bool) -> PluginInvocationResponse {
    if let Ok(resp) = serde_json::from_str::<PluginInvocationResponse>(stdout.trim()) {
        return resp;
    }
    if let Some(resp) = stdout
        .lines()
        .rev()
        .map(str::trim)
        .filter(|l| l.starts_with('{'))
        .find_map(|l| serde_json::from_str::<PluginInvocationResponse>(l).ok())
    {
        return resp;
    }
    PluginInvocationResponse {
        ok: process_ok,
        output: stdout.trim_end().to_string(),
    }
}

/// Runs a plugin process with `CLIBM_PROJECT_ROOT` set, optionally writing `stdin_payload`
/// and enforcing `timeout`. Returns (exit success, stdout). Stderr goes to the terminal.
fn run_plugin_process(
    exe: &Path,
    args: &[String],
    project_root: &Path,
    stdin_payload: Option<&[u8]>,
    timeout: Option<Duration>,
    in_hook: bool,
) -> Result<(bool, String)> {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(project_root)
        .env("CLIBM_PROJECT_ROOT", project_root)
        .stdin(if stdin_payload.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if in_hook {
        cmd.env("CLIBM_IN_HOOK", "1");
    }
    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to start plugin binary '{}'", exe.display()))?;
    if let (Some(payload), Some(mut stdin)) = (stdin_payload, child.stdin.take()) {
        let _ = stdin.write_all(payload);
    }
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });

    let status = match timeout {
        None => child.wait()?,
        Some(limit) => {
            let start = Instant::now();
            loop {
                if let Some(status) = child.try_wait()? {
                    break status;
                }
                if start.elapsed() >= limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!("timed out after {}s", limit.as_secs());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };
    let out = reader.join().unwrap_or_default();
    Ok((status.success(), String::from_utf8_lossy(&out).to_string()))
}

pub(crate) fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}{:x}", std::process::id(), nanos & 0xffff_ffff)
}

/// Deletes leftovers named `<exe_name>.old.*` from earlier locked-file rotations.
fn cleanup_rotated_binaries(dir: &Path, exe_name: &str) {
    let prefix = format!("{}.old.", exe_name);
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

/// Copies `src` over `dest`. When Windows keeps `dest` locked (the binary is running),
/// the old file is renamed out of the way first.
pub fn replace_binary(src: &Path, dest: &Path) -> Result<()> {
    if dest.exists() && fs::remove_file(dest).is_err() {
        let name = dest
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let rotated = dest.with_file_name(format!("{}.old.{}", name, unique_suffix()));
        fs::rename(dest, &rotated).with_context(|| {
            format!("Cannot replace {} (is it running?)", dest.display())
        })?;
    }
    fs::copy(src, dest).with_context(|| format!("Failed to copy binary to {}", dest.display()))?;
    if let Some(dir) = dest.parent() {
        if let Some(name) = dest.file_name() {
            cleanup_rotated_binaries(dir, &name.to_string_lossy());
        }
    }
    Ok(())
}
