//! Extensibility & Plugin Contract System (Architectural Principle #5 — 100% Pure Core)
//!
//! `clibm_core` is 100% pure and contains zero built-in plugins.
//! All plugins (`clibm-dep-audit`, `clibm-export-md`, `clibm-workflow-lint`,
//! `clibm-mcp`, `clibm-docs`, `clibm-tui`, `clibm-web`) live as standalone
//! Cargo crates in `plugins/` and communicate via `CLIBM_PLUGIN_CONTRACT_V1`.

use crate::cas::CasStore;
use crate::models::*;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub hooks: Vec<String>,
    /// Path or binary name of the installed standalone plugin executable on Linux/Windows.
    pub executable: Option<String>,
    #[serde(flatten, default)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationRequest {
    pub contract: String,
    pub event: String,
    pub tree_hash: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationResponse {
    pub ok: bool,
    pub output: String,
}

pub struct PluginManager;

impl PluginManager {
    /// Returns an empty list (`vec![]`) because `clibm_core` is 100% pure by default.
    /// Plugins are installed from their respective folders in `plugins/`.
    pub fn default_manifests() -> Vec<PluginManifest> {
        Vec::new()
    }

    /// Returns the 3 default plugin manifests (`clibm-dep-audit`, `clibm-export-md`, `clibm-workflow-lint`)
    /// when the user explicitly chooses the "Core + Default Plugins" profile during installation.
    pub fn default_plugin_bundle(install_dir: &str) -> Vec<PluginManifest> {
        let base = install_dir.trim_end_matches('/');
        vec![
            PluginManifest {
                name: "clibm-dep-audit".to_string(),
                version: "0.1.0".to_string(),
                description: "Critical path and circular dependency analyzer for task graphs"
                    .to_string(),
                enabled: true,
                hooks: vec!["pre-commit".to_string(), "on-task-depend".to_string()],
                executable: Some(format!("{}/clibm-dep-audit", base)),
                extra: {
                    let mut m = std::collections::BTreeMap::new();
                    m.insert(
                        "tools".to_string(),
                        serde_json::json!([
                            {
                                "name": "audit_dependencies",
                                "description": "Audit task dependency DAG in computed index to detect circular chains and blockers",
                                "input_schema": {
                                    "type": "object",
                                    "properties": {}
                                },
                                "is_mutating": false,
                                "execution_ref": "audit"
                            }
                        ]),
                    );
                    m
                },
            },
            PluginManifest {
                name: "clibm-export-md".to_string(),
                version: "0.1.0".to_string(),
                description: "Generates portable Markdown sprint reports from CAS snapshots"
                    .to_string(),
                enabled: true,
                hooks: vec!["on-export".to_string()],
                executable: Some(format!("{}/clibm-export-md", base)),
                extra: {
                    let mut m = std::collections::BTreeMap::new();
                    m.insert(
                        "tools".to_string(),
                        serde_json::json!([
                            {
                                "name": "export_project",
                                "description": "Generate portable Markdown sprint report from CAS object snapshot",
                                "input_schema": {
                                    "type": "object",
                                    "properties": {}
                                },
                                "is_mutating": false,
                                "execution_ref": "export"
                            }
                        ]),
                    );
                    m
                },
            },
            PluginManifest {
                name: "clibm-workflow-lint".to_string(),
                version: "0.1.0".to_string(),
                description: "Validates Kanban WIP limits, unblocked dependencies, and SLA dates"
                    .to_string(),
                enabled: true,
                hooks: vec!["pre-move".to_string(), "pre-commit".to_string()],
                executable: Some(format!("{}/clibm-workflow-lint", base)),
                extra: {
                    let mut m = std::collections::BTreeMap::new();
                    m.insert(
                        "tools".to_string(),
                        serde_json::json!([
                            {
                                "name": "lint_workflow",
                                "description": "Validate Kanban column WIP limits and required assignees",
                                "input_schema": {
                                    "type": "object",
                                    "properties": {}
                                },
                                "is_mutating": false,
                                "execution_ref": "lint"
                            }
                        ]),
                    );
                    m
                },
            },
        ]
    }

    /// Returns the user-level global plugin registry path (`~/.clibm/plugins/registry.json`)
    /// so plugins installed once from their own folder are available in all ongoing projects.
    pub fn global_registry_file() -> Option<PathBuf> {
        let home = env::var("CLIBM_HOME")
            .or_else(|_| env::var("HOME"))
            .or_else(|_| env::var("USERPROFILE"))
            .ok()?;
        let dir = PathBuf::from(home).join(".clibm").join("plugins");
        let _ = fs::create_dir_all(&dir);
        Some(dir.join("registry.json"))
    }

    /// Returns candidate `registry.json` files located alongside installed binaries
    /// (`<install_path>/registry.json`, `<current_exe_dir>/registry.json`, `~/.clibm-bin/registry.json`, `~/.local/bin/registry.json`).
    pub fn extra_registry_files(cas: &CasStore) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let home = env::var("CLIBM_HOME")
            .or_else(|_| env::var("HOME"))
            .or_else(|_| env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from);

        if let Ok(cfg) = cas.load_config() {
            if !cfg.install_path.is_empty() {
                let expanded = if let Some(ref h) = home {
                    cfg.install_path.replace('~', &h.to_string_lossy())
                } else {
                    cfg.install_path.clone()
                };
                files.push(PathBuf::from(expanded).join("registry.json"));
            }
        }
        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                files.push(dir.join("registry.json"));
            }
        }
        if let Some(h) = home {
            files.push(h.join(".clibm-bin").join("registry.json"));
            files.push(h.join(".local").join("bin").join("registry.json"));
        }
        files
    }

    /// Resolves short plugin aliases to their standalone crate binary names in `plugins/`.
    pub fn normalize_plugin_name(name: &str) -> String {
        match name.trim() {
            "tui" => "clibm-tui".to_string(),
            "web" => "clibm-web".to_string(),
            "mcp" => "clibm-mcp".to_string(),
            "docs" => "clibm-docs".to_string(),
            "dep-audit" => "clibm-dep-audit".to_string(),
            "export-md" => "clibm-export-md".to_string(),
            "workflow-lint" => "clibm-workflow-lint".to_string(),
            other => other.to_string(),
        }
    }

    /// Checks whether a binary exists in configured `install_path`, `~/.local/bin`, `~/.cargo/bin`, `~/.clibm-bin`, or `PATH`.
    pub fn find_installed_binary(bin_name: &str) -> Option<PathBuf> {
        let exe_name = if cfg!(windows) && !bin_name.ends_with(".exe") {
            format!("{}.exe", bin_name)
        } else {
            bin_name.to_string()
        };

        let explicit = PathBuf::from(&exe_name);
        if explicit.is_absolute() && explicit.exists() {
            return Some(explicit);
        }

        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join(&exe_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        if let Ok(home) = env::var("CLIBM_HOME")
            .or_else(|_| env::var("HOME"))
            .or_else(|_| env::var("USERPROFILE"))
        {
            let home_path = PathBuf::from(home);
            let candidates = [
                home_path.join(".local").join("bin").join(&exe_name),
                home_path.join(".cargo").join("bin").join(&exe_name),
                home_path.join(".clibm-bin").join(&exe_name),
            ];
            for c in candidates {
                if c.exists() {
                    return Some(c);
                }
            }
        }

        if let Ok(path_var) = env::var("PATH") {
            for dir in env::split_paths(&path_var) {
                let full = dir.join(&exe_name);
                if full.exists() {
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

    /// Loads the merged plugin registry (project `.clibm/plugins/registry.json` +
    /// global `~/.clibm/plugins/registry.json` + binary directory `registry.json`).
    /// Starts empty on a Pure Core repository and persists merged manifests into `.clibm/plugins/registry.json`.
    pub fn load_registry(cas: &CasStore) -> Result<Vec<PluginManifest>> {
        let reg_file = cas.plugins_dir().join("registry.json");
        let had_reg_file = reg_file.exists();
        let mut plugins = if had_reg_file {
            let content = read_utf8_strip_bom(&reg_file)?;
            serde_json::from_str::<Vec<PluginManifest>>(&content)
                .unwrap_or_else(|_| Self::default_manifests())
        } else {
            Self::default_manifests()
        };

        let mut changed = !had_reg_file;
        let mut candidate_files = Vec::new();
        if let Some(global_file) = Self::global_registry_file() {
            candidate_files.push(global_file);
        }
        candidate_files.extend(Self::extra_registry_files(cas));

        for ext_file in candidate_files {
            if ext_file.exists() && ext_file != reg_file {
                if let Ok(content) = read_utf8_strip_bom(&ext_file) {
                    if let Ok(ext_plugins) =
                        serde_json::from_str::<Vec<PluginManifest>>(&content)
                    {
                        for gp in ext_plugins {
                            if let Some(existing) =
                                plugins.iter_mut().find(|p| p.name == gp.name)
                            {
                                if existing.executable.is_none() && gp.executable.is_some() {
                                    existing.executable = gp.executable;
                                    changed = true;
                                }
                            } else {
                                plugins.push(gp);
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        if cas.is_initialized() && changed {
            let _ = Self::save_registry(cas, &plugins);
        }

        Ok(plugins)
    }

    pub fn save_registry(cas: &CasStore, plugins: &[PluginManifest]) -> Result<()> {
        if cas.is_initialized() {
            let _ = fs::create_dir_all(cas.plugins_dir());
            let reg_file = cas.plugins_dir().join("registry.json");
            write_utf8_file(&reg_file, &serde_json::to_string_pretty(plugins)?)?;
        }
        Ok(())
    }

    pub fn save_manifest_to_file(reg_file: &Path, manifest: &PluginManifest) -> Result<()> {
        if let Some(parent) = reg_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut list: Vec<PluginManifest> = if reg_file.exists() {
            read_utf8_strip_bom(reg_file)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(existing) = list.iter_mut().find(|p| p.name == manifest.name) {
            *existing = manifest.clone();
        } else {
            list.push(manifest.clone());
        }
        write_utf8_file(reg_file, &serde_json::to_string_pretty(&list)?)?;
        Ok(())
    }

    pub fn save_global_manifest(manifest: PluginManifest) -> Result<()> {
        if let Some(global_file) = Self::global_registry_file() {
            Self::save_manifest_to_file(&global_file, &manifest)?;
        }
        Ok(())
    }

    /// Resolves an official plugin's remote directory and source file list for raw GitHub installation.
    fn official_remote_plugin_spec(raw_target: &str) -> Option<(&'static str, &'static str, Vec<&'static str>)> {
        let leaf = Path::new(raw_target)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(raw_target);
        let norm = Self::normalize_plugin_name(leaf);
        match norm.as_str() {
            "clibm-mcp" => Some((
                "clibm-mcp",
                "plugins/clibm-mcp",
                vec![
                    "Cargo.toml",
                    "src/main.rs",
                    "src/config.rs",
                    "src/error.rs",
                    "src/protocol/mod.rs",
                    "src/protocol/types.rs",
                    "src/protocol/schema.rs",
                    "src/backend/mod.rs",
                    "src/backend/bridge.rs",
                    "src/adapter/mod.rs",
                    "src/adapter/dispatcher.rs",
                    "src/adapter/permissions.rs",
                    "src/adapter/tools.rs",
                    "src/adapter/resources.rs",
                    "src/adapter/prompts.rs",
                    "src/transport/mod.rs",
                    "src/transport/stdio.rs",
                    "src/transport/http.rs",
                ],
            )),
            "clibm-docs" => Some(("clibm-docs", "plugins/clibm-docs", vec!["Cargo.toml", "src/main.rs"])),
            "clibm-dep-audit" => Some(("clibm-dep-audit", "plugins/clibm-dep-audit", vec!["Cargo.toml", "src/main.rs"])),
            "clibm-export-md" => Some(("clibm-export-md", "plugins/clibm-export-md", vec!["Cargo.toml", "src/main.rs"])),
            "clibm-workflow-lint" => Some(("clibm-workflow-lint", "plugins/clibm-workflow-lint", vec!["Cargo.toml", "src/main.rs"])),
            "clibm-tui" => Some(("clibm-tui", "plugins/clibm-tui", vec!["Cargo.toml", "src/main.rs"])),
            "clibm-web" => Some(("clibm-web", "plugins/clibm-web", vec!["Cargo.toml", "src/main.rs", "src/ui.html"])),
            "plugin-template" | "clibm-plugin-template" => Some((
                "clibm-plugin-template",
                "plugins/plugin-template",
                vec!["Cargo.toml", "src/main.rs"],
            )),
            _ => None,
        }
    }

    fn download_remote_file(url: &str, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let status = if cfg!(windows) {
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &format!(
                        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                        url.replace('\'', "''"),
                        dest.display().to_string().replace('\'', "''")
                    ),
                ])
                .status()
                .context("Failed to invoke PowerShell Invoke-WebRequest")?
        } else {
            Command::new("curl")
                .args(["-fsSL", url, "-o"])
                .arg(dest)
                .status()
                .context("Failed to invoke curl for remote plugin source download")?
        };
        if !status.success() {
            bail!("Failed to download remote plugin file from {}", url);
        }
        Ok(())
    }

    /// Builds and installs a standalone plugin from its own folder in `plugins/` (or fetches from
    /// raw GitHub if not present locally), copies its binary to the configured `install_path`
    /// (or `~/.local/bin` / `~/.clibm-bin`), and registers it in the plugin registry.
    pub fn install_plugin_from_dir(
        cas: &CasStore,
        plugin_dir: &Path,
        mcp_transport: Option<&str>,
    ) -> Result<PluginManifest> {
        let mut resolved_dir = plugin_dir.to_path_buf();
        let mut temp_cleanup_dir: Option<PathBuf> = None;

        if !resolved_dir.join("Cargo.toml").exists() {
            let raw_str = plugin_dir.to_string_lossy();
            if let Some((crate_name, remote_subpath, files)) = Self::official_remote_plugin_spec(&raw_str) {
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let local_candidates = [
                    cwd.join(remote_subpath),
                    cwd.join("rust-clibm").join(remote_subpath),
                ];
                if let Some(found_local) = local_candidates.into_iter().find(|p| p.join("Cargo.toml").exists()) {
                    resolved_dir = found_local;
                } else {
                    let raw_base = env::var("CLIBM_RAW_BASE").unwrap_or_else(|_| {
                        "https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm".to_string()
                    });
                    let base_url = format!("{}/{}", raw_base.trim_end_matches('/'), remote_subpath);
                    let tmp_dir = env::temp_dir().join(format!(
                        "{}-remote-build-{}",
                        crate_name,
                        std::process::id()
                    ));
                    let _ = fs::remove_dir_all(&tmp_dir);
                    fs::create_dir_all(&tmp_dir)?;
                    println!(
                        "==> [Remote Mode] Fetching '{}' from {}...",
                        crate_name, base_url
                    );
                    for rel in files {
                        let url = format!("{}/{}", base_url, rel);
                        let dest = tmp_dir.join(rel);
                        Self::download_remote_file(&url, &dest)?;
                    }
                    resolved_dir = tmp_dir.clone();
                    temp_cleanup_dir = Some(tmp_dir);
                }
            }
        }

        let cargo_toml = resolved_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            bail!(
                "No Cargo.toml found in '{}'. Provide a valid plugin folder path or an official plugin name (e.g. `clibm plugin install tui`).",
                plugin_dir.display()
            );
        }

        let toml_text = read_utf8_strip_bom(&cargo_toml)?;
        let name = toml_text
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .strip_prefix("name")
                    .and_then(|r| r.trim().strip_prefix('='))
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
            .ok_or_else(|| anyhow!("Could not parse package name from {}", cargo_toml.display()))?;

        let description = toml_text
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .strip_prefix("description")
                    .and_then(|r| r.trim().strip_prefix('='))
                    .map(|v| v.trim().trim_matches('"').to_string())
            })
            .unwrap_or_else(|| format!("Installed CLIBM standalone plugin ({})", name));

        println!("==> Building plugin '{}' in {}...", name, resolved_dir.display());
        let mut cargo_args = vec!["build", "--release"];
        if name == "clibm-mcp" {
            match mcp_transport.unwrap_or("both") {
                "stdio" => {
                    cargo_args.push("--no-default-features");
                    cargo_args.push("--features");
                    cargo_args.push("transport-stdio");
                }
                "http" => {
                    cargo_args.push("--no-default-features");
                    cargo_args.push("--features");
                    cargo_args.push("transport-http");
                }
                _ => {
                    cargo_args.push("--features");
                    cargo_args.push("transport-stdio,transport-http");
                }
            }
        }
        let status = Command::new("cargo")
            .args(&cargo_args)
            .current_dir(&resolved_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        let status = match status {
            Ok(s) => s,
            Err(e) => {
                if let Some(tmp) = &temp_cleanup_dir {
                    let _ = fs::remove_dir_all(tmp);
                }
                return Err(e).context("Failed to invoke `cargo build --release` in plugin folder");
            }
        };

        if !status.success() {
            if let Some(tmp) = &temp_cleanup_dir {
                let _ = fs::remove_dir_all(tmp);
            }
            bail!("Plugin build failed for '{}'", name);
        }

        let bin_filename = if cfg!(windows) {
            format!("{}.exe", name)
        } else {
            name.clone()
        };
        let candidates = [
            resolved_dir.join("target").join("release").join(&bin_filename),
            resolved_dir
                .join("..")
                .join("..")
                .join("target")
                .join("release")
                .join(&bin_filename),
            env::current_dir()
                .unwrap_or_default()
                .join("target")
                .join("release")
                .join(&bin_filename),
        ];
        let built_bin = match candidates.into_iter().find(|p| p.exists()) {
            Some(p) => p,
            None => {
                if let Some(tmp) = &temp_cleanup_dir {
                    let _ = fs::remove_dir_all(tmp);
                }
                bail!(
                    "Expected release binary '{}' not found in local or workspace target/release directories",
                    bin_filename
                );
            }
        };

        let cfg = cas.load_config().unwrap_or_default();
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let install_dir = if !cfg.install_path.is_empty() && cfg.install_path != "~/.local/bin" {
            PathBuf::from(cfg.install_path.replace('~', &home))
        } else if cfg!(windows) {
            PathBuf::from(&home).join(".clibm-bin")
        } else {
            PathBuf::from(&home).join(".local").join("bin")
        };

        fs::create_dir_all(&install_dir)?;
        let installed_bin = install_dir.join(&bin_filename);
        fs::copy(&built_bin, &installed_bin)?;

        if let Some(tmp) = temp_cleanup_dir {
            let _ = fs::remove_dir_all(tmp);
        } else {
            // Post-install space optimization: clean build artifacts to save disk space
            let local_target = resolved_dir.join("target");
            if local_target.exists() && local_target.is_dir() {
                let _ = fs::remove_dir_all(&local_target);
                println!(
                    "==> [Space Optimization] Cleaned intermediate build artifacts in {}",
                    local_target.display()
                );
            } else {
                let _ = Command::new("cargo")
                    .args(&["clean", "-p", &name, "--release"])
                    .current_dir(&resolved_dir)
                    .status();
            }
        }

        if !Self::is_dir_on_env_path(&install_dir) {
            println!(
                "==> Note: '{}' is not currently on your user PATH. Run the plugin's install script (`install.ps1` / `install.sh`) or add '{}' to your user PATH to invoke '{}' directly.",
                install_dir.display(),
                install_dir.display(),
                name
            );
        }

        let hooks = if name.contains("tui") {
            vec!["interactive-tui".to_string(), "on-board-move".to_string()]
        } else if name.contains("web") {
            vec!["http-bridge".to_string(), "on-state-query".to_string()]
        } else if name.contains("mcp") {
            let mode = mcp_transport.unwrap_or("both");
            if let Ok(mut updated_cfg) = cas.load_config() {
                updated_cfg.mcp_transport = mode.to_string();
                let _ = cas.save_config(&updated_cfg);
            }
            match mode {
                "stdio" => vec!["mcp-stdio".to_string(), "on-ai-tool-call".to_string()],
                "http" => vec!["mcp-http-sse".to_string(), "on-ai-tool-call".to_string()],
                _ => vec![
                    "mcp-stdio".to_string(),
                    "mcp-http-sse".to_string(),
                    "on-ai-tool-call".to_string(),
                ],
            }
        } else if name.contains("docs") {
            vec!["ai-docs-generator".to_string(), "on-init".to_string()]
        } else if name.contains("dep-audit") {
            vec!["pre-commit".to_string(), "on-task-depend".to_string()]
        } else if name.contains("export-md") {
            vec!["on-export".to_string()]
        } else if name.contains("workflow-lint") {
            vec!["pre-move".to_string(), "pre-commit".to_string()]
        } else {
            vec!["manual-run".to_string()]
        };

        let mut manifest = PluginManifest {
            name: name.clone(),
            version: "0.1.0".to_string(),
            description,
            enabled: true,
            hooks,
            executable: Some(installed_bin.to_string_lossy().to_string()),
            extra: std::collections::BTreeMap::new(),
        };

        if cas.is_initialized() {
            let mut project_plugins = Self::load_registry(cas)?;
            if let Some(existing) = project_plugins.iter_mut().find(|p| p.name == name) {
                manifest.extra = existing.extra.clone();
                *existing = manifest.clone();
            } else {
                project_plugins.push(manifest.clone());
            }
            Self::save_registry(cas, &project_plugins)?;
        }

        Self::save_global_manifest(manifest.clone())?;

        Ok(manifest)
    }

    /// Scaffolds a new standalone plugin crate in `plugins/<crate_name>` using the official
    /// CLIBM plugin template (`Cargo.toml`, `src/main.rs`, `README.md`, `install.sh`, `install.ps1`, `uninstall.sh`, `uninstall.ps1`)
    /// with UTF-8 encoding and registers it in the active repository's plugin registry.
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
//! Subscribed Hooks: {:?}

use anyhow::Result;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {{
    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    println!("[{crate_name}] Running in ongoing CLIBM project: {{}}", project_root.display());
    Ok(())
}}
"#,
            hooks
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
            "#!/usr/bin/env bash\n\
# Encoding: UTF-8 (without BOM)\n\
set -euo pipefail\n\
{sh_helper_block}\n\
OWNER_DIRS=$(clibm_path_owner_dirs \"{crate_name}\" || true)\n\
SEARCH_DIRS=$(\n\
  {{\n\
    [ -n \"$OWNER_DIRS\" ] && printf '%s\\n' \"$OWNER_DIRS\"\n\
    printf '%s\\n' \"$HOME/.local/bin\" \"$HOME/.cargo/bin\" \"$HOME/.clibm-bin\"\n\
  }} | awk 'NF && !seen[$0]++'\n\
)\n\
while IFS= read -r dir; do\n\
  [ -z \"$dir\" ] && continue\n\
  if [ -f \"${{dir}}/{crate_name}\" ]; then\n\
    rm -f \"${{dir}}/{crate_name}\"\n\
    echo \"==> Removed binary: ${{dir}}/{crate_name}\"\n\
  fi\n\
done <<EOF\n\
$SEARCH_DIRS\n\
EOF\n\
clibm_unregister_plugin \"{crate_name}\" \"$(pwd)\"\n\
clibm_path_remove \"{crate_name}\"\n\
if [ -n \"$CLIBM_REMOVED_PATH_DIRS\" ]; then\n\
  while IFS= read -r rdir; do\n\
    [ -z \"$rdir\" ] && continue\n\
    echo \"==> Removed user PATH entry: $rdir\"\n\
    if [ -d \"$rdir\" ] && [ -z \"$(ls -A \"$rdir\" 2>/dev/null || true)\" ]; then\n\
      rmdir \"$rdir\" 2>/dev/null || true\n\
    fi\n\
  done <<EOF\n\
$CLIBM_REMOVED_PATH_DIRS\n\
EOF\n\
fi\n\
echo \"Uninstalled {crate_name}\"\n"
        );
        write_utf8_file(&plugin_dir.join("uninstall.sh"), &uninstall_sh)?;

        let uninstall_ps1 = format!(
            "# Encoding: UTF-8 (without BOM)\n\
$ErrorActionPreference = \"Stop\"\n\
{ps1_helper_block}\n\
$OwnerDirs = @(Get-ClibmOwnerDirs -Owner \"{crate_name}\")\n\
$SearchDirs = @()\n\
$Seen = @{{}}\n\
foreach ($D in ($OwnerDirs + @((Join-Path $HOME \".clibm-bin\"), (Join-Path $HOME \".local\\bin\"), (Join-Path $HOME \".cargo\\bin\")))) {{\n\
  if ([string]::IsNullOrWhiteSpace($D)) {{ continue }}\n\
  $K = ($D -replace '/', '\\').TrimEnd('\\').ToLowerInvariant()\n\
  if (-not $Seen.ContainsKey($K)) {{ $Seen[$K] = $true; $SearchDirs += $D }}\n\
}}\n\
foreach ($Dir in $SearchDirs) {{\n\
  $Bin = Join-Path $Dir \"{crate_name}.exe\"\n\
  if (Test-Path -LiteralPath $Bin) {{\n\
    Remove-Item -LiteralPath $Bin -Force\n\
    Write-Host \"==> Removed binary: $Bin\"\n\
  }}\n\
}}\n\
Unregister-ClibmPlugin -Name \"{crate_name}\" -ProjectDir (Get-Location).ProviderPath -ExtraDirs $SearchDirs\n\
$RemovedPathDirs = @(Remove-ClibmPathEntry -Owner \"{crate_name}\")\n\
foreach ($RDir in $RemovedPathDirs) {{\n\
  Write-Host \"==> Removed user PATH entry: $RDir\"\n\
  if ((Test-Path -LiteralPath $RDir) -and @(Get-ChildItem -LiteralPath $RDir -Force -ErrorAction SilentlyContinue).Count -eq 0) {{\n\
    Remove-Item -LiteralPath $RDir -Force -ErrorAction SilentlyContinue\n\
  }}\n\
}}\n\
Write-Host \"Uninstalled {crate_name}.exe\"\n"
        );
        write_utf8_file(&plugin_dir.join("uninstall.ps1"), &uninstall_ps1)?;

        let manifest = PluginManifest {
            name: crate_name.clone(),
            version: "0.1.0".to_string(),
            description: description.to_string(),
            enabled: true,
            hooks: if hooks.is_empty() {
                vec!["manual-run".to_string()]
            } else {
                hooks
            },
            executable: Some(format!("~/.local/bin/{}", crate_name)),
            extra: std::collections::BTreeMap::new(),
        };

        let _ = Self::save_global_manifest(manifest.clone());

        if cas.is_initialized() {
            let mut list = Self::load_registry(cas)?;
            if let Some(existing) = list.iter_mut().find(|p| p.name == crate_name) {
                *existing = manifest.clone();
            } else {
                list.push(manifest.clone());
            }
            Self::save_registry(cas, &list)?;
        }

        Ok((plugin_dir, manifest))
    }

    pub fn set_plugin_enabled(cas: &CasStore, name: &str, enabled: bool) -> Result<()> {
        let canonical_name = Self::normalize_plugin_name(name);
        let mut plugins = Self::load_registry(cas)?;
        let found = plugins.iter_mut().find(|p| p.name == canonical_name);
        match found {
            Some(p) => {
                p.enabled = enabled;
                if p.executable.is_some() {
                    let _ = Self::save_global_manifest(p.clone());
                }
                Self::save_registry(cas, &plugins)
            }
            None => bail!("Plugin '{}' is not registered", canonical_name),
        }
    }

    /// Executes an installed standalone plugin binary against the ongoing project in `cas`.
    /// Because `clibm_core` is 100% pure, all plugins must be installed from `plugins/<name>` first.
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
                    "Plugin '{}' is not installed in this pure core repository. Install it from `plugins/{}` via `cd plugins/{} && ./install.sh` or `clibm plugin install plugins/{}`.",
                    canonical_name,
                    canonical_name,
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

        let exe = manifest.executable.as_ref().ok_or_else(|| {
            anyhow!(
                "Plugin '{}' has no executable binary registered. Re-install it from `plugins/{}`.",
                canonical_name,
                canonical_name
            )
        })?;

        let resolved_exe = Self::find_installed_binary(exe)
            .or_else(|| Self::find_installed_binary(&canonical_name))
            .unwrap_or_else(|| PathBuf::from(exe));

        let project_root = cas.workspace_root();
        let is_interactive = canonical_name == "clibm-tui"
            || canonical_name == "clibm-web"
            || canonical_name == "clibm-mcp"
            || manifest.hooks.iter().any(|h| {
                h == "interactive-tui"
                    || h == "http-bridge"
                    || h == "mcp-stdio"
                    || h == "mcp-http-sse"
            });

        if is_interactive {
            let status = Command::new(&resolved_exe)
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
                        resolved_exe.display(),
                        project_root.display()
                    )
                })?;

            return Ok(PluginInvocationResponse {
                ok: status.success(),
                output: format!(
                    "[{}] Session finished in project {}",
                    canonical_name,
                    project_root.display()
                ),
            });
        }

        // Standard JSON stdin/stdout contract for non-interactive external plugin binaries
        let req = PluginInvocationRequest {
            contract: CONTRACT_VERSION.to_string(),
            event: "manual-run".to_string(),
            tree_hash: tree_hash.to_string(),
            args: args.to_vec(),
        };
        let mut child = Command::new(&resolved_exe)
            .args(args)
            .current_dir(&project_root)
            .env("CLIBM_PROJECT_ROOT", &project_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&serde_json::to_vec(&req)?)?;
        }
        let out = child.wait_with_output()?;
        if let Ok(resp) = serde_json::from_slice::<PluginInvocationResponse>(&out.stdout) {
            return Ok(resp);
        }
        Ok(PluginInvocationResponse {
            ok: out.status.success(),
            output: String::from_utf8_lossy(&out.stdout).to_string(),
        })
    }
}
