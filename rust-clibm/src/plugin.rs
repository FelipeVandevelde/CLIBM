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
    $HomeDir = if ($HOME) {{ $HOME }} elseif ($env:USERPROFILE) {{ $env:USERPROFILE }} else {{ [Environment]::GetFolderPath('UserProfile') }}\n\
    $LibDir = Join-Path $HomeDir '.clibm\\lib'\n\
    $HelperFile = Join-Path $LibDir 'env-path.ps1'\n\
    $EmbeddedVersion = {ver}\n\
    $EmbeddedContent = @'\n\
{clean}\n\
'@\n\
    $NeedsWrite = $true\n\
    if (Test-Path -LiteralPath $HelperFile) {{\n\
        try {{\n\
            $HeadLines = Get-Content -LiteralPath $HelperFile -TotalCount 5 -Encoding UTF8 -ErrorAction Stop\n\
            foreach ($Line in $HeadLines) {{\n\
                if ($Line -match 'clibm-env-path-version:\\s*(\\d+)') {{\n\
                    if ([int]$Matches[1] -ge $EmbeddedVersion) {{\n\
                        $NeedsWrite = $false\n\
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
        $Utf8Bom = New-Object System.Text.UTF8Encoding($true)\n\
        [System.IO.File]::WriteAllText($HelperFile, $EmbeddedContent, $Utf8Bom)\n\
    }}\n\
    return $HelperFile\n\
}}\n\
. (Initialize-ClibmEnvPathHelper)\n\
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
  local lib_dir=\"${{HOME}}/.clibm/lib\"\n\
  local helper_file=\"${{lib_dir}}/env-path.sh\"\n\
  local embedded_version={ver}\n\
  local needs_write=1\n\
  local bom=$'\\xef\\xbb\\xbf'\n\
  if [ -f \"$helper_file\" ]; then\n\
    local first_three\n\
    first_three=$(head -c 3 \"$helper_file\" 2>/dev/null || true)\n\
    local existing_ver\n\
    existing_ver=$(tr -d '\\r' < \"$helper_file\" | sed \"1s/^${{bom}}//\" | sed -n '1,5s/^# clibm-env-path-version:[[:space:]]*\\([0-9][0-9]*\\).*/\\1/p' | head -n1)\n\
    if [ \"$first_three\" != \"$bom\" ] && [ -n \"$existing_ver\" ] && [ \"$existing_ver\" -ge \"$embedded_version\" ] 2>/dev/null; then\n\
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

fn write_utf8_bom_file(path: &Path, content: &str) -> Result<()> {
    let clean = content.strip_prefix(UTF8_BOM).unwrap_or(content);
    let with_bom = format!("{}{}", UTF8_BOM, clean);
    fs::write(path, with_bom.as_bytes())?;
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
            },
            PluginManifest {
                name: "clibm-export-md".to_string(),
                version: "0.1.0".to_string(),
                description: "Generates portable Markdown sprint reports from CAS snapshots"
                    .to_string(),
                enabled: true,
                hooks: vec!["on-export".to_string()],
                executable: Some(format!("{}/clibm-export-md", base)),
            },
            PluginManifest {
                name: "clibm-workflow-lint".to_string(),
                version: "0.1.0".to_string(),
                description: "Validates Kanban WIP limits, unblocked dependencies, and SLA dates"
                    .to_string(),
                enabled: true,
                hooks: vec!["pre-move".to_string(), "pre-commit".to_string()],
                executable: Some(format!("{}/clibm-workflow-lint", base)),
            },
        ]
    }

    /// Returns the user-level global plugin registry path (`~/.clibm/plugins/registry.json`)
    /// so plugins installed once from their own folder are available in all ongoing projects.
    pub fn global_registry_file() -> Option<PathBuf> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .ok()?;
        let dir = PathBuf::from(home).join(".clibm").join("plugins");
        let _ = fs::create_dir_all(&dir);
        Some(dir.join("registry.json"))
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

        if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
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
    /// global `~/.clibm/plugins/registry.json`). Starts empty on a Pure Core repository.
    pub fn load_registry(cas: &CasStore) -> Result<Vec<PluginManifest>> {
        let reg_file = cas.plugins_dir().join("registry.json");
        let mut plugins = if reg_file.exists() {
            let content = read_utf8_strip_bom(&reg_file)?;
            serde_json::from_str::<Vec<PluginManifest>>(&content)
                .unwrap_or_else(|_| Self::default_manifests())
        } else {
            let defaults = Self::default_manifests();
            if cas.is_initialized() {
                let _ = Self::save_registry(cas, &defaults);
            }
            defaults
        };

        // Merge global registry (`~/.clibm/plugins/registry.json`)
        if let Some(global_file) = Self::global_registry_file() {
            if global_file.exists() {
                if let Ok(content) = read_utf8_strip_bom(&global_file) {
                    if let Ok(global_plugins) =
                        serde_json::from_str::<Vec<PluginManifest>>(&content)
                    {
                        for gp in global_plugins {
                            if let Some(existing) =
                                plugins.iter_mut().find(|p| p.name == gp.name)
                            {
                                if existing.executable.is_none() && gp.executable.is_some() {
                                    existing.executable = gp.executable;
                                }
                            } else {
                                plugins.push(gp);
                            }
                        }
                    }
                }
            }
        }

        Ok(plugins)
    }

    pub fn save_registry(cas: &CasStore, plugins: &[PluginManifest]) -> Result<()> {
        if cas.is_initialized() {
            let _ = fs::create_dir_all(cas.plugins_dir());
            let reg_file = cas.plugins_dir().join("registry.json");
            write_utf8_bom_file(&reg_file, &serde_json::to_string_pretty(plugins)?)?;
        }
        Ok(())
    }

    pub fn save_global_manifest(manifest: PluginManifest) -> Result<()> {
        if let Some(global_file) = Self::global_registry_file() {
            let mut global_list: Vec<PluginManifest> = if global_file.exists() {
                read_utf8_strip_bom(&global_file)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            if let Some(existing) = global_list.iter_mut().find(|p| p.name == manifest.name) {
                *existing = manifest;
            } else {
                global_list.push(manifest);
            }
            write_utf8_bom_file(&global_file, &serde_json::to_string_pretty(&global_list)?)?;
        }
        Ok(())
    }

    /// Builds and installs a standalone plugin from its own folder in `plugins/`,
    /// copies its binary to the configured `install_path` (or `~/.local/bin`),
    /// and registers it in the plugin registry.
    pub fn install_plugin_from_dir(
        cas: &CasStore,
        plugin_dir: &Path,
        mcp_transport: Option<&str>,
    ) -> Result<PluginManifest> {
        let cargo_toml = plugin_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            bail!(
                "No Cargo.toml found in '{}'. Plugins must be installed from within their own plugin folder.",
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

        println!("==> Building plugin '{}' in {}...", name, plugin_dir.display());
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(plugin_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to invoke `cargo build --release` in plugin folder")?;

        if !status.success() {
            bail!("Plugin build failed for '{}'", name);
        }

        let bin_filename = if cfg!(windows) {
            format!("{}.exe", name)
        } else {
            name.clone()
        };
        let built_bin = plugin_dir.join("target").join("release").join(&bin_filename);
        if !built_bin.exists() {
            bail!("Expected release binary not found at {}", built_bin.display());
        }

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

        let manifest = PluginManifest {
            name: name.clone(),
            version: "0.1.0".to_string(),
            description,
            enabled: true,
            hooks,
            executable: Some(installed_bin.to_string_lossy().to_string()),
        };

        Self::save_global_manifest(manifest.clone())?;

        if cas.is_initialized() {
            let mut project_plugins = Self::load_registry(cas)?;
            if let Some(existing) = project_plugins.iter_mut().find(|p| p.name == name) {
                *existing = manifest.clone();
            } else {
                project_plugins.push(manifest.clone());
            }
            Self::save_registry(cas, &project_plugins)?;
        }

        Ok(manifest)
    }

    /// Scaffolds a new standalone plugin crate in `plugins/<crate_name>` using the official
    /// CLIBM plugin template (`Cargo.toml`, `src/main.rs`, `README.md`, `install.sh`, `install.ps1`, `uninstall.sh`, `uninstall.ps1`)
    /// with UTF-8-BOM (`EF BB BF`) encoding and registers it in the active repository's plugin registry.
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
        write_utf8_bom_file(&plugin_dir.join("Cargo.toml"), &cargo_toml)?;

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
        write_utf8_bom_file(&src_dir.join("main.rs"), &main_rs)?;

        let readme = format!(
            "# {crate_name}\n\n{description}\n\n- **Encoding:** UTF-8 with BOM (`EF BB BF`)\n\n## Build & Install (From This Folder)\n\n```bash\ncd plugins/{crate_name}\n./install.sh\n# Or on Windows PowerShell:\n.\\install.ps1\n```\n\n## Run in Any Ongoing CLIBM Project\n\n```bash\nclibm plugin run {crate_name}\n```\n\n## Uninstall\n\n```bash\n./uninstall.sh\n# Or on Windows PowerShell:\n.\\uninstall.ps1\n```\n"
        );
        write_utf8_bom_file(&plugin_dir.join("README.md"), &readme)?;

        let ps1_helper_block = build_embedded_ps1_helper_block();
        let sh_helper_block = build_embedded_sh_helper_block();

        let install_sh = format!(
            "#!/usr/bin/env bash\n\
# Encoding: UTF-8 with BOM (EF BB BF)\n\
set -euo pipefail\n\
cd \"$(dirname \"$0\")\"\n\
HOOK_MODE=\"\"\n\
INSTALL_DIR=\"${{INSTALL_DIR:-}}\"\n\
NO_MODIFY_PATH=false\n\
NON_INTERACTIVE=false\n\
while [[ $# -gt 0 ]]; do\n\
  case \"$1\" in\n\
    --hooks|-h) HOOK_MODE=\"$2\"; shift 2 ;;\n\
    --install-dir|--path) INSTALL_DIR=\"$2\"; shift 2 ;;\n\
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
    echo \"[1/2] Select Hook Subscription Mode for {crate_name}:\"\n\
    echo \"  1) Default Hooks [Default]\"\n\
    echo \"  2) Manual Only\"\n\
    read -r -p \"Select option [1-2, default: 1]: \" c || true\n\
    case \"${{c:-1}}\" in 2) HOOK_MODE=\"manual-only\" ;; *) HOOK_MODE=\"default-hooks\" ;; esac\n\
  fi\n\
fi\n\
if [ -z \"$INSTALL_DIR\" ]; then\n\
  if [ \"$NON_INTERACTIVE\" = true ]; then\n\
    INSTALL_DIR=\"$HOME/.local/bin\"\n\
  else\n\
    read -r -p \"[2/2] Specify install directory [default: $HOME/.local/bin]: \" d || true\n\
    INSTALL_DIR=\"${{d:-$HOME/.local/bin}}\"\n\
  fi\n\
fi\n\
INSTALL_DIR=$(clibm_normalize_dir \"$INSTALL_DIR\")\n\
if ! clibm_core_installed \"$INSTALL_DIR\"; then\n\
  echo \"==> Warning: CLIBM core installation not detected. {crate_name} depends on CLIBM core at runtime; proceeding with standalone plugin installation...\"\n\
fi\n\
cargo build --release\n\
mkdir -p \"$INSTALL_DIR\"\n\
cp \"target/release/{crate_name}\" \"$INSTALL_DIR/{crate_name}\"\n\
chmod +x \"$INSTALL_DIR/{crate_name}\"\n\
if [ \"$NO_MODIFY_PATH\" = false ]; then\n\
  clibm_path_add \"{crate_name}\" \"$INSTALL_DIR\"\n\
  echo \"==> User PATH ($INSTALL_DIR): $CLIBM_LAST_PATH_STATUS\"\n\
else\n\
  echo \"==> User PATH: skipped (--no-modify-path)\"\n\
fi\n\
echo \"Installed {crate_name} to $INSTALL_DIR/{crate_name} (hooks: $HOOK_MODE)\"\n"
        );
        write_utf8_bom_file(&plugin_dir.join("install.sh"), &install_sh)?;

        let install_ps1 = format!(
            "# Encoding: UTF-8 with BOM (EF BB BF)\n\
param(\n\
    [string]$Hooks = \"\",\n\
    [string]$InstallDir = \"\",\n\
    [switch]$NoModifyPath = $false,\n\
    [switch]$Yes = $false\n\
)\n\
$ErrorActionPreference = \"Stop\"\n\
Set-Location -Path $PSScriptRoot\n\
{ps1_helper_block}\n\
if (-not $Hooks) {{\n\
  if ($Yes) {{ $Hooks = \"default-hooks\" }} else {{\n\
    $c = Read-Host \"[1/2] Select Hook Mode for {crate_name} (1=default-hooks [Default], 2=manual-only)\"\n\
    if ($c -eq \"2\") {{ $Hooks = \"manual-only\" }} else {{ $Hooks = \"default-hooks\" }}\n\
  }}\n\
}}\n\
$DefaultBin = Join-Path $HOME \".clibm-bin\"\n\
if (-not $InstallDir) {{\n\
  if ($Yes) {{ $InstallDir = $DefaultBin }} else {{\n\
    $d = Read-Host \"[2/2] Specify install directory [default: $DefaultBin]\"\n\
    if ([string]::IsNullOrWhiteSpace($d)) {{ $InstallDir = $DefaultBin }} else {{ $InstallDir = $d }}\n\
  }}\n\
}}\n\
$InstallDir = Resolve-ClibmNormalizedDir $InstallDir\n\
if (-not (Test-ClibmCoreInstalled -ExtraDir $InstallDir)) {{\n\
  Write-Host \"==> Warning: CLIBM core installation not detected. {crate_name} depends on CLIBM core at runtime; proceeding with standalone plugin installation...\" -ForegroundColor Yellow\n\
}}\n\
cargo build --release\n\
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null\n\
Copy-Item \"target\\release\\{crate_name}.exe\" -Destination (Join-Path $InstallDir \"{crate_name}.exe\") -Force\n\
if (-not $NoModifyPath) {{\n\
  $PathRes = Add-ClibmPathEntry -Owner \"{crate_name}\" -Dir $InstallDir\n\
  Write-Host \"==> User PATH ($InstallDir): $($PathRes.Status)\"\n\
}} else {{\n\
  Write-Host \"==> User PATH: skipped (-NoModifyPath)\"\n\
}}\n\
Write-Host \"Installed {crate_name}.exe to $InstallDir (hooks: $Hooks)\"\n"
        );
        write_utf8_bom_file(&plugin_dir.join("install.ps1"), &install_ps1)?;

        let uninstall_sh = format!(
            "#!/usr/bin/env bash\n\
# Encoding: UTF-8 with BOM (EF BB BF)\n\
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
        write_utf8_bom_file(&plugin_dir.join("uninstall.sh"), &uninstall_sh)?;

        let uninstall_ps1 = format!(
            "# Encoding: UTF-8 with BOM (EF BB BF)\n\
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
$RemovedPathDirs = @(Remove-ClibmPathEntry -Owner \"{crate_name}\")\n\
foreach ($RDir in $RemovedPathDirs) {{\n\
  Write-Host \"==> Removed user PATH entry: $RDir\"\n\
  if ((Test-Path -LiteralPath $RDir) -and @(Get-ChildItem -LiteralPath $RDir -Force -ErrorAction SilentlyContinue).Count -eq 0) {{\n\
    Remove-Item -LiteralPath $RDir -Force -ErrorAction SilentlyContinue\n\
  }}\n\
}}\n\
Write-Host \"Uninstalled {crate_name}.exe\"\n"
        );
        write_utf8_bom_file(&plugin_dir.join("uninstall.ps1"), &uninstall_ps1)?;

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
        };

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
