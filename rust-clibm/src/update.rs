//! `clibm update`: rebuilds CLIBM (and optionally its installed plugins) from GitHub or
//! from a local clone, and replaces the installed binaries in place.
//!
//! Builds use `cargo install --root <temp>`, which resolves the whole Cargo workspace
//! (so plugins that depend on the core build correctly), and the result is copied over
//! the installed binary. A running binary that Windows keeps locked is renamed out of
//! the way first.

use crate::install::InstallConfig;
use crate::models::ClibmConfig;
use crate::plugin::{replace_binary, unique_suffix, PluginManager, PluginManifest};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Official repository, used when nothing else is configured.
pub const DEFAULT_REPO_URL: &str = "https://github.com/FelipeVandevelde/CLIBM.git";
pub const DEFAULT_BRANCH: &str = "main";

/// A repository URL in the form `cargo install --git` accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoRef {
    /// Clone URL, e.g. `https://github.com/OWNER/REPO.git`.
    pub clone_url: String,
    /// Branch found in a pasted browser URL (`.../tree/<branch>/...`).
    pub branch_hint: Option<String>,
    /// `(owner, repo)` for GitHub repositories.
    pub github: Option<(String, String)>,
}

/// Normalizes what users paste as a repository URL. GitHub browser URLs such as
/// `https://github.com/OWNER/REPO/tree/main/rust-clibm` become
/// `https://github.com/OWNER/REPO.git` (branch `main`). Other `https://`, `ssh://`,
/// `git://`, `file://` and `git@host:` URLs are kept as they are.
pub fn normalize_repo_url(input: &str) -> Result<RepoRef> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("Repository URL cannot be empty");
    }
    let lower = trimmed.to_ascii_lowercase();
    let github_prefixes = [
        "https://github.com/",
        "http://github.com/",
        "https://www.github.com/",
        "github.com/",
        "git@github.com:",
        "ssh://git@github.com/",
    ];
    if let Some(prefix) = github_prefixes.iter().find(|p| lower.starts_with(*p)) {
        let rest = &trimmed[prefix.len()..];
        let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() < 2 {
            bail!(
                "GitHub URL '{}' must include the owner and the repository (https://github.com/OWNER/REPO)",
                input
            );
        }
        let owner = parts[0];
        let repo = parts[1].trim_end_matches(".git");
        let valid = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        if !valid(owner) || !valid(repo) {
            bail!("'{}' is not a valid GitHub repository URL", input);
        }
        let branch_hint = if parts.len() >= 4 && matches!(parts[2], "tree" | "blob") {
            Some(parts[3].to_string())
        } else {
            None
        };
        return Ok(RepoRef {
            clone_url: format!("https://github.com/{}/{}.git", owner, repo),
            branch_hint,
            github: Some((owner.to_string(), repo.to_string())),
        });
    }
    let is_url = ["https://", "http://", "ssh://", "git://", "file://"]
        .iter()
        .any(|p| lower.starts_with(p))
        || (lower.starts_with("git@") && lower.contains(':'));
    if is_url {
        return Ok(RepoRef {
            clone_url: trimmed.to_string(),
            branch_hint: None,
            github: None,
        });
    }
    bail!(
        "'{}' is not a repository URL (expected e.g. https://github.com/OWNER/REPO.git)",
        input
    )
}

/// Where `clibm update` builds from.
#[derive(Debug, Clone)]
pub enum UpdateSource {
    Git { repo: RepoRef, branch: String },
    /// A local clone (the repository root or its `rust-clibm` folder).
    Path(PathBuf),
}

impl UpdateSource {
    /// Picks the source: explicit flags first, then the installer's record
    /// (`~/.clibm/install-config.json`), then the project config, then the defaults.
    pub fn resolve(
        repo_flag: Option<&str>,
        branch_flag: Option<&str>,
        path_flag: Option<&str>,
        project_cfg: Option<&ClibmConfig>,
        install: &InstallConfig,
    ) -> Result<Self> {
        if let Some(p) = path_flag {
            let path = PathBuf::from(p);
            let root = workspace_dir(&path).ok_or_else(|| {
                anyhow!(
                    "'{}' is not a CLIBM source folder (no rust-clibm/Cargo.toml or Cargo.toml found)",
                    path.display()
                )
            })?;
            return Ok(UpdateSource::Path(root));
        }
        // Installed from a local clone: keep updating from that clone.
        if repo_flag.is_none() && branch_flag.is_none() {
            if let Some(local) = install
                .source_repo
                .as_deref()
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
                .and_then(|p| workspace_dir(&p))
            {
                return Ok(UpdateSource::Path(local));
            }
        }
        let default_project_url = ClibmConfig::default().github_repo_url;
        let project_url = project_cfg
            .map(|c| c.github_repo_url.clone())
            .filter(|u| !u.trim().is_empty() && *u != default_project_url);
        let repo_input = repo_flag
            .map(str::to_string)
            .or_else(|| install.source_repo.clone())
            .or(project_url)
            .unwrap_or_else(|| DEFAULT_REPO_URL.to_string());
        let repo = normalize_repo_url(&repo_input)?;
        let branch = branch_flag
            .map(str::to_string)
            .or_else(|| repo.branch_hint.clone())
            .or_else(|| install.source_ref.clone())
            .or_else(|| {
                project_cfg
                    .map(|c| c.update_branch.clone())
                    .filter(|b| !b.trim().is_empty())
            })
            .unwrap_or_else(|| DEFAULT_BRANCH.to_string());
        Ok(UpdateSource::Git { repo, branch })
    }

    pub fn describe(&self) -> String {
        match self {
            UpdateSource::Git { repo, branch } => format!("{} (branch {})", repo.clone_url, branch),
            UpdateSource::Path(p) => format!("local source {}", p.display()),
        }
    }
}

/// `<dir>/rust-clibm` when it exists, else `<dir>` when it holds the workspace.
fn workspace_dir(dir: &Path) -> Option<PathBuf> {
    let nested = dir.join("rust-clibm");
    if nested.join("Cargo.toml").is_file() {
        return Some(nested);
    }
    if dir.join("Cargo.toml").is_file() && dir.join("src").join("main.rs").is_file() {
        return Some(dir.to_path_buf());
    }
    None
}

/// Folder of an official plugin crate inside `rust-clibm/plugins`.
pub fn official_plugin_folder(name: &str) -> Option<&'static str> {
    match name {
        "clibm-mcp" => Some("clibm-mcp"),
        "clibm-tui" => Some("clibm-tui"),
        "clibm-web" => Some("clibm-web"),
        "clibm-dep-audit" => Some("clibm-dep-audit"),
        "clibm-export-md" => Some("clibm-export-md"),
        "clibm-workflow-lint" => Some("clibm-workflow-lint"),
        "clibm-plugin-template" => Some("plugin-template"),
        _ => None,
    }
}

/// Cargo feature flags that reproduce an installed MCP build (its transports are
/// recorded as `mcp-stdio` / `mcp-http` hooks).
fn mcp_feature_args(manifest: &PluginManifest) -> Vec<String> {
    let stdio = manifest.subscribes_to("mcp-stdio");
    let http = manifest.subscribes_to("mcp-http") || manifest.subscribes_to("mcp-http-sse");
    match (stdio, http) {
        (true, false) => mcp_transport_args("stdio"),
        (false, true) => mcp_transport_args("http"),
        _ => mcp_transport_args("both"),
    }
}

/// Removes its directory when dropped.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A freshly built binary waiting to be installed.
pub struct BuiltBinary {
    pub path: PathBuf,
    pub commit: Option<String>,
    _root: TempDir,
}

fn find_commit_in(text: &str) -> Option<String> {
    let idx = text.find('#')?;
    let candidate: String = text[idx + 1..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    (candidate.len() == 40).then_some(candidate)
}

fn git_head(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && sha.len() == 40).then_some(sha)
}

/// Where a crate is built from.
#[derive(Debug, Clone)]
pub enum CrateLocation {
    Git { url: String, branch: String },
    /// Folder that contains the crate's `Cargo.toml`.
    Dir(PathBuf),
}

/// Builds `crate_name` with `cargo install --root <temp> --locked`, which resolves the
/// whole Cargo workspace (path dependencies included) in a throw-away directory.
pub fn cargo_install(location: &CrateLocation, crate_name: &str, extra_args: &[String]) -> Result<BuiltBinary> {
    let root = env::temp_dir().join(format!("clibm-build-{}-{}", crate_name, unique_suffix()));
    fs::create_dir_all(&root)?;
    let guard = TempDir(root.clone());

    let mut cmd = Command::new("cargo");
    cmd.arg("install").arg("--root").arg(&root).arg("--locked").arg("--force");
    // Having <root>/bin on PATH stops cargo from telling the user to add the temporary
    // folder to their PATH.
    let mut paths = vec![root.join("bin")];
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }
    if let Ok(joined) = env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
    let mut local_commit = None;
    let described = match location {
        CrateLocation::Git { url, branch } => {
            cmd.args(["--git", url, "--branch", branch, crate_name]);
            format!("{} (branch {})", url, branch)
        }
        CrateLocation::Dir(dir) => {
            cmd.arg("--path").arg(dir);
            local_commit = git_head(dir);
            dir.display().to_string()
        }
    };
    cmd.args(extra_args);

    println!("==> Building {} from {}...", crate_name, described);
    let status = cmd
        .status()
        .context("Failed to run `cargo` (install Rust from https://rustup.rs)")?;
    if !status.success() {
        bail!("Building {} failed (cargo exit code {:?})", crate_name, status.code());
    }

    let path = root
        .join("bin")
        .join(PluginManager::binary_file_name(crate_name));
    if !path.is_file() {
        bail!("cargo did not produce {}", path.display());
    }
    let commit = local_commit.or_else(|| {
        fs::read_to_string(root.join(".crates2.json"))
            .ok()
            .and_then(|t| find_commit_in(&t))
    });
    Ok(BuiltBinary {
        path,
        commit,
        _root: guard,
    })
}

/// Builds an official crate (`clibm` or a plugin) from an update source.
pub fn build_crate(source: &UpdateSource, crate_name: &str, extra_args: &[String]) -> Result<BuiltBinary> {
    let location = match source {
        UpdateSource::Git { repo, branch } => CrateLocation::Git {
            url: repo.clone_url.clone(),
            branch: branch.clone(),
        },
        UpdateSource::Path(workspace) => {
            if crate_name == "clibm" {
                CrateLocation::Dir(workspace.clone())
            } else {
                let folder = official_plugin_folder(crate_name)
                    .ok_or_else(|| anyhow!("'{}' is not an official CLIBM plugin", crate_name))?;
                CrateLocation::Dir(workspace.join("plugins").join(folder))
            }
        }
    };
    cargo_install(&location, crate_name, extra_args)
}

/// Feature flags for an MCP build with the given transport (`stdio`, `http` or `both`).
pub fn mcp_transport_args(transport: &str) -> Vec<String> {
    match transport {
        "stdio" => vec!["--no-default-features".into(), "--features".into(), "transport-stdio".into()],
        "http" => vec!["--no-default-features".into(), "--features".into(), "transport-http".into()],
        _ => Vec::new(),
    }
}

/// Downloads `url` as text with curl/wget (Unix) or PowerShell (Windows).
pub fn fetch_text(url: &str) -> Result<String> {
    let output = if cfg!(windows) {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
                     $ProgressPreference = 'SilentlyContinue'; \
                     [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
                     (Invoke-WebRequest -UseBasicParsing -Headers @{{ 'User-Agent' = 'clibm-updater' }} -Uri '{}').Content",
                    url.replace('\'', "''")
                ),
            ])
            .output()
            .context("Failed to run PowerShell to reach the network")?
    } else {
        match Command::new("curl")
            .args(["-fsSL", "-A", "clibm-updater", url])
            .output()
        {
            Ok(out) => out,
            Err(_) => Command::new("wget")
                .args(["-qO-", url])
                .output()
                .context("Neither curl nor wget is available")?,
        }
    };
    if !output.status.success() {
        bail!(
            "Could not download {}: {}",
            url,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Latest commit of `branch` on the remote.
pub fn remote_commit(repo: &RepoRef, branch: &str) -> Result<String> {
    if let Some((owner, name)) = &repo.github {
        let body = fetch_text(&format!(
            "https://api.github.com/repos/{}/{}/commits/{}",
            owner, name, branch
        ))?;
        let json: serde_json::Value = serde_json::from_str(body.trim())
            .with_context(|| format!("Unexpected answer from the GitHub API for {}/{}", owner, name))?;
        return json
            .get("sha")
            .and_then(|s| s.as_str())
            .map(str::to_string)
            .ok_or_else(|| anyhow!("Branch '{}' was not found in {}/{}", branch, owner, name));
    }
    let out = Command::new("git")
        .args(["ls-remote", &repo.clone_url, &format!("refs/heads/{}", branch)])
        .output()
        .context("Checking a non-GitHub repository needs `git`")?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace()
        .next()
        .filter(|s| s.len() == 40)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Branch '{}' was not found in {}", branch, repo.clone_url))
}

/// Version in the remote `rust-clibm/Cargo.toml` (GitHub only).
pub fn remote_version(repo: &RepoRef, branch: &str) -> Option<String> {
    let (owner, name) = repo.github.as_ref()?;
    let text = fetch_text(&format!(
        "https://raw.githubusercontent.com/{}/{}/{}/rust-clibm/Cargo.toml",
        owner, name, branch
    ))
    .ok()?;
    parse_package_version(&text)
}

fn parse_package_version(cargo_toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo_toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = t.strip_prefix("version") {
                if let Some(v) = rest.trim().strip_prefix('=') {
                    return Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// Result of `clibm update --check`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheck {
    pub source: String,
    pub installed_version: String,
    pub installed_commit: Option<String>,
    pub latest_version: Option<String>,
    pub latest_commit: String,
    pub update_available: bool,
}

pub fn check(source: &UpdateSource, install: &InstallConfig) -> Result<UpdateCheck> {
    let (latest_commit, latest_version) = match source {
        UpdateSource::Git { repo, branch } => (remote_commit(repo, branch)?, remote_version(repo, branch)),
        UpdateSource::Path(workspace) => {
            let head = git_head(workspace).ok_or_else(|| {
                anyhow!(
                    "Cannot read the current commit of {} (is it a git clone and is git installed?)",
                    workspace.display()
                )
            })?;
            let version = fs::read_to_string(workspace.join("Cargo.toml"))
                .ok()
                .and_then(|t| parse_package_version(&t));
            (head, version)
        }
    };
    let installed_version = env!("CARGO_PKG_VERSION").to_string();
    let installed_commit = install.source_commit.clone();
    let update_available = match &installed_commit {
        Some(c) => c != &latest_commit,
        None => true,
    } || latest_version
        .as_deref()
        .map(|v| v != installed_version)
        .unwrap_or(false);
    Ok(UpdateCheck {
        source: source.describe(),
        installed_version,
        installed_commit,
        latest_version,
        latest_commit,
        update_available,
    })
}

/// Rebuilds the core and replaces the running `clibm` binary. Returns the new commit.
pub fn update_core(source: &UpdateSource) -> Result<Option<String>> {
    let built = build_crate(source, "clibm", &[])?;
    let dest = env::current_exe().context("Cannot locate the running clibm binary")?;
    replace_binary(&built.path, &dest)?;
    println!("==> Installed {}", dest.display());

    let mut install = InstallConfig::load();
    match source {
        UpdateSource::Git { repo, branch } => {
            install.source_repo = Some(repo.clone_url.clone());
            install.source_ref = Some(branch.clone());
        }
        UpdateSource::Path(p) => {
            install.source_repo = Some(p.display().to_string());
            install.source_ref = None;
        }
    }
    install.source_commit = built.commit.clone();
    install.version = installed_version_of(&dest);
    if install.install_path.is_none() {
        install.install_path = dest.parent().map(|p| p.display().to_string());
    }
    install.save()?;
    Ok(built.commit)
}

fn installed_version_of(exe: &Path) -> Option<String> {
    let out = Command::new(exe).arg("--version").output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .last()
        .map(str::to_string)
}

/// What happened to one plugin during `clibm update --plugins`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUpdate {
    pub name: String,
    pub updated: bool,
    pub message: String,
}

/// Rebuilds every installed official plugin in place, keeping its hooks, its
/// enabled flag and (for MCP) its transports.
pub fn update_plugins(source: &UpdateSource) -> Vec<PluginUpdate> {
    let mut results = Vec::new();
    for manifest in PluginManager::load_global_registry() {
        let Some(dest) = manifest.executable_path().filter(|p| p.is_file()) else {
            continue;
        };
        if official_plugin_folder(&manifest.name).is_none() {
            results.push(PluginUpdate {
                name: manifest.name.clone(),
                updated: false,
                message: "not an official plugin; update it from its own source".to_string(),
            });
            continue;
        }
        let extra = if manifest.name == "clibm-mcp" {
            mcp_feature_args(&manifest)
        } else {
            Vec::new()
        };
        let outcome = build_crate(source, &manifest.name, &extra).and_then(|built| {
            replace_binary(&built.path, &dest)?;
            let hooks = manifest.hooks.join(",");
            let mut fresh = PluginManager::query_manifest(&dest, Some(&hooks))
                .unwrap_or_else(|| manifest.clone());
            fresh.enabled = manifest.enabled;
            PluginManager::register_installed_plugin(&fresh)?;
            Ok(())
        });
        results.push(match outcome {
            Ok(()) => PluginUpdate {
                name: manifest.name.clone(),
                updated: true,
                message: format!("updated {}", dest.display()),
            },
            Err(e) => PluginUpdate {
                name: manifest.name.clone(),
                updated: false,
                message: e.to_string(),
            },
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_urls_become_clone_urls() {
        let r = normalize_repo_url("https://github.com/FelipeVandevelde/CLIBM/tree/main/rust-clibm").unwrap();
        assert_eq!(r.clone_url, "https://github.com/FelipeVandevelde/CLIBM.git");
        assert_eq!(r.branch_hint.as_deref(), Some("main"));
        assert_eq!(r.github, Some(("FelipeVandevelde".into(), "CLIBM".into())));

        for input in [
            "https://github.com/FelipeVandevelde/CLIBM",
            "https://github.com/FelipeVandevelde/CLIBM.git",
            "https://github.com/FelipeVandevelde/CLIBM/",
            "github.com/FelipeVandevelde/CLIBM",
            "git@github.com:FelipeVandevelde/CLIBM.git",
        ] {
            let r = normalize_repo_url(input).unwrap();
            assert_eq!(r.clone_url, "https://github.com/FelipeVandevelde/CLIBM.git", "{}", input);
            assert_eq!(r.branch_hint, None, "{}", input);
        }
    }

    #[test]
    fn other_urls_are_kept_and_junk_is_rejected() {
        let r = normalize_repo_url("https://gitlab.com/me/clibm.git").unwrap();
        assert_eq!(r.clone_url, "https://gitlab.com/me/clibm.git");
        assert!(r.github.is_none());
        assert!(normalize_repo_url("").is_err());
        assert!(normalize_repo_url("not a url").is_err());
        assert!(normalize_repo_url("https://github.com/onlyowner").is_err());
    }

    #[test]
    fn source_precedence() {
        let install = InstallConfig {
            source_repo: Some("https://github.com/a/b.git".into()),
            source_ref: Some("dev".into()),
            ..Default::default()
        };
        let mut cfg = ClibmConfig::default();
        cfg.github_repo_url = "https://github.com/FelipeVandevelde/CLIBM/tree/main/rust-clibm".into();

        // Flags win.
        match UpdateSource::resolve(Some("https://github.com/x/y"), Some("feat"), None, Some(&cfg), &install).unwrap() {
            UpdateSource::Git { repo, branch } => {
                assert_eq!(repo.clone_url, "https://github.com/x/y.git");
                assert_eq!(branch, "feat");
            }
            _ => panic!("expected git source"),
        }
        // Then the installer's record.
        match UpdateSource::resolve(None, None, None, Some(&cfg), &install).unwrap() {
            UpdateSource::Git { repo, branch } => {
                assert_eq!(repo.clone_url, "https://github.com/a/b.git");
                assert_eq!(branch, "dev");
            }
            _ => panic!("expected git source"),
        }
        // Then a (broken, browser-style) project URL, which is repaired.
        match UpdateSource::resolve(None, None, None, Some(&cfg), &InstallConfig::default()).unwrap() {
            UpdateSource::Git { repo, branch } => {
                assert_eq!(repo.clone_url, "https://github.com/FelipeVandevelde/CLIBM.git");
                assert_eq!(branch, "main");
            }
            _ => panic!("expected git source"),
        }
    }

    #[test]
    fn parses_versions_and_commits() {
        assert_eq!(
            parse_package_version("[workspace]\nmembers=[]\n[package]\nname = \"clibm\"\nversion = \"0.2.0\"\n"),
            Some("0.2.0".into())
        );
        let crates2 = r#"{"installs":{"clibm 0.1.0 (git+https://github.com/a/b.git?branch=main#0123456789abcdef0123456789abcdef01234567)":{}}}"#;
        assert_eq!(
            find_commit_in(crates2).as_deref(),
            Some("0123456789abcdef0123456789abcdef01234567")
        );
    }
}

#[cfg(test)]
mod local_source_tests {
    use super::*;

    #[test]
    fn local_install_source_updates_from_that_clone() {
        // The repository this test runs in is a valid local source.
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let install = InstallConfig {
            source_repo: Some(here.display().to_string()),
            ..Default::default()
        };
        match UpdateSource::resolve(None, None, None, None, &install).unwrap() {
            UpdateSource::Path(p) => assert_eq!(p, here),
            other => panic!("expected local source, got {:?}", other),
        }
        // An explicit --repo still wins.
        assert!(matches!(
            UpdateSource::resolve(Some("https://github.com/x/y"), None, None, None, &install).unwrap(),
            UpdateSource::Git { .. }
        ));
    }
}
