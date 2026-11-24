//! Content-Addressable Storage (CAS) Engine
//!
//! Stores immutable objects on the local filesystem inside `.clibm/objects/<shard>/<rest>.json`.
//! Implements section 2.2 of the CLIBM v0.1.0 specification:
//! - JSON as serialization format
//! - BLAKE3 as cryptographic hashing algorithm
//! - Immutable objects identified by hashes (`Type`, `Version`, `Hash`, `Payload`)
//! - References (`HEAD` -> `refs/heads/main`) marking the latest commit of the history
//!   (CLIBM keeps one straight line of commits; other refs only exist in old repositories)
//! - Configuration management (`.clibm/config.json`) with `auto_commit = true` by default

use crate::canonical::{compute_object_hash, normalize_value};
use crate::models::{ClibmConfig, CommitPayload, OBJ_COMMIT};
use anyhow::{anyhow, bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const FORMAT_VERSION: &str = "0.1.0";

/// Specification 2.2.2: Immutable Object Envelope
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CasObject {
    #[serde(rename = "Type")]
    pub object_type: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Hash")]
    pub hash: String,
    #[serde(rename = "Payload")]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsckReport {
    pub total_objects: usize,
    pub verified_objects: usize,
    pub corrupted_hashes: Vec<String>,
    /// Objects written by old clibm-web / clibm-tui builds, which hashed
    /// `"type:version:payload"` instead of the canonical envelope. Their content
    /// is intact; only the hashing scheme differs.
    #[serde(default)]
    pub legacy_hashes: Vec<String>,
}

/// Hash scheme used by clibm-web / clibm-tui before they moved onto `clibm_core`.
fn legacy_plugin_hash(object_type: &str, version: &str, payload: &Value) -> Option<String> {
    let canonical_payload = serde_json::to_string(&normalize_value(payload)).ok()?;
    let preimage = format!("{}:{}:{}", object_type, version, canonical_payload);
    Some(blake3::hash(preimage.as_bytes()).to_hex().to_string())
}

/// Validates a ref name before it is used as a path under `.clibm/refs/heads/`, so a
/// user-supplied string (e.g. `../../x`) never becomes a path outside it.
/// Allows `[A-Za-z0-9._/-]`, rejects `..`, empty segments, a leading `-`, `/` or `.`,
/// a trailing `/` or `.lock`, and the reserved name `HEAD`.
pub fn validate_ref_name(name: &str) -> Result<()> {
    let invalid = |reason: &str| -> Result<()> {
        bail!("Invalid ref name '{}': {}", name, reason)
    };
    if name.is_empty() || name.len() > 100 {
        return invalid("must be 1-100 characters long");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
    {
        return invalid("only letters, digits, '.', '_', '-' and '/' are allowed");
    }
    if name.eq_ignore_ascii_case("HEAD") {
        return invalid("'HEAD' is reserved");
    }
    if name.starts_with('-') || name.starts_with('/') || name.ends_with('/') {
        return invalid("cannot start with '-' or '/', or end with '/'");
    }
    if name.contains("..") || name.contains("//") {
        return invalid("cannot contain '..' or '//'");
    }
    if name.ends_with(".lock") {
        return invalid("cannot end with '.lock'");
    }
    if name.split('/').any(|seg| seg.is_empty() || seg.starts_with('.')) {
        return invalid("path segments cannot be empty or start with '.'");
    }
    Ok(())
}

/// Manages `.clibm/` filesystem layout cleanly across Linux and Windows.
#[derive(Debug, Clone)]
pub struct CasStore {
    root_dir: PathBuf,
}

impl CasStore {
    /// Opens or targets `.clibm` directly inside `workspace_root`, or discovers an
    /// existing ongoing `.clibm` repository in `workspace_root` or its parent directories.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        if let Ok(env_root) = env::var("CLIBM_PROJECT_ROOT") {
            let env_path = PathBuf::from(env_root);
            let candidate = env_path.join(".clibm");
            if candidate.exists() {
                return Self { root_dir: candidate };
            }
        }

        let start = workspace_root.as_ref();
        for ancestor in start.ancestors() {
            let candidate = ancestor.join(".clibm");
            if candidate.exists() && candidate.join("objects").exists() {
                return Self { root_dir: candidate };
            }
        }

        Self {
            root_dir: start.join(".clibm"),
        }
    }

    /// Targets `<workspace_root>/.clibm` exactly, without consulting `CLIBM_PROJECT_ROOT`
    /// or searching parent directories. Used by `clibm init` and destructive operations
    /// that must never act on a repository discovered in an ancestor folder.
    pub fn at_root(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            root_dir: workspace_root.as_ref().join(".clibm"),
        }
    }

    pub fn clibm_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn workspace_root(&self) -> PathBuf {
        self.root_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.root_dir.join("objects")
    }

    pub fn refs_heads_dir(&self) -> PathBuf {
        self.root_dir.join("refs").join("heads")
    }

    pub fn index_dir(&self) -> PathBuf {
        self.root_dir.join("index")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.root_dir.join("plugins")
    }

    pub fn head_file(&self) -> PathBuf {
        self.root_dir.join("HEAD")
    }

    pub fn stage_file(&self) -> PathBuf {
        self.root_dir.join("STAGE_TREE")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root_dir.join("config.json")
    }

    /// Initializes the `.clibm` directory tree and default `.clibm/config.json` (`auto_commit = true`).
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.objects_dir())
            .context("Failed to create .clibm/objects directory")?;
        fs::create_dir_all(self.refs_heads_dir())
            .context("Failed to create .clibm/refs/heads directory")?;
        fs::create_dir_all(self.index_dir())
            .context("Failed to create .clibm/index directory")?;
        fs::create_dir_all(self.plugins_dir())
            .context("Failed to create .clibm/plugins directory")?;

        if !self.head_file().exists() {
            fs::write(self.head_file(), "ref: refs/heads/main\n")
                .context("Failed to initialize .clibm/HEAD")?;
        }
        if !self.config_file().exists() {
            self.save_config(&ClibmConfig::default())?;
        }
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.root_dir.exists() && self.objects_dir().exists()
    }

    pub fn load_config(&self) -> Result<ClibmConfig> {
        let cfg_file = self.config_file();
        if !cfg_file.exists() {
            let def = ClibmConfig::default();
            if self.is_initialized() {
                let _ = self.save_config(&def);
            }
            return Ok(def);
        }
        let raw = fs::read_to_string(&cfg_file).context("Failed to read .clibm/config.json")?;
        let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
        Ok(serde_json::from_str(content).unwrap_or_default())
    }

    pub fn save_config(&self, cfg: &ClibmConfig) -> Result<()> {
        if let Some(parent) = self.config_file().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(self.config_file(), serde_json::to_string_pretty(cfg)?)
            .context("Failed to write .clibm/config.json")?;
        Ok(())
    }

    /// Maps a 64-char BLAKE3 hex hash to `.clibm/objects/<first-2>/<remaining-62>.json`
    fn object_path(&self, hash: &str) -> Result<PathBuf> {
        if hash.len() < 6 {
            bail!("Invalid BLAKE3 object hash: {}", hash);
        }
        let (shard, rest) = hash.split_at(2);
        Ok(self.objects_dir().join(shard).join(format!("{}.json", rest)))
    }

    /// Resolves a short hash prefix (e.g. 8 chars) or full 64-char hash.
    pub fn resolve_hash_prefix(&self, prefix: &str) -> Result<String> {
        let clean = prefix.trim().to_lowercase();
        if clean.len() == 64 {
            let path = self.object_path(&clean)?;
            if path.exists() {
                return Ok(clean);
            }
            bail!("Object {} not found in CAS", clean);
        }
        if clean.len() < 4 {
            bail!("Hash prefix must be at least 4 hex characters: {}", prefix);
        }

        let (shard, rest_prefix) = clean.split_at(2);
        let shard_dir = self.objects_dir().join(shard);
        if !shard_dir.exists() {
            bail!("No CAS object matching prefix {}", prefix);
        }

        let mut matches = Vec::new();
        for entry in fs::read_dir(shard_dir)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = file_name.strip_suffix(".json") {
                if stem.starts_with(rest_prefix) {
                    matches.push(format!("{}{}", shard, stem));
                }
            }
        }

        match matches.len() {
            0 => bail!("No CAS object found matching prefix '{}'", prefix),
            1 => Ok(matches.remove(0)),
            n => bail!("Ambiguous hash prefix '{}' matches {} objects", prefix, n),
        }
    }

    /// Writes an immutable object to CAS and returns its 64-char BLAKE3 hash.
    /// Idempotent: if the hash already exists on disk, skips rewriting.
    pub fn put_object<T: Serialize>(&self, object_type: &str, payload: &T) -> Result<String> {
        let (hash, normalized_payload) =
            compute_object_hash(object_type, FORMAT_VERSION, payload)?;

        let path = self.object_path(&hash)?;
        if path.exists() {
            return Ok(hash);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let envelope = CasObject {
            object_type: object_type.to_string(),
            version: FORMAT_VERSION.to_string(),
            hash: hash.clone(),
            payload: normalized_payload,
        };

        let pretty_json = serde_json::to_string_pretty(&envelope)?;
        fs::write(&path, pretty_json)
            .with_context(|| format!("Failed to write CAS object {}", hash))?;

        Ok(hash)
    }

    /// Reads and verifies a raw `CasObject` envelope by full or prefix hash.
    pub fn get_raw_object(&self, hash_or_prefix: &str) -> Result<CasObject> {
        let full_hash = self.resolve_hash_prefix(hash_or_prefix)?;
        let path = self.object_path(&full_hash)?;
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read CAS object {}", full_hash))?;
        let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
        let obj: CasObject = serde_json::from_str(content)
            .with_context(|| format!("Corrupt JSON in CAS object {}", full_hash))?;
        Ok(obj)
    }

    /// Reads an object and deserializes its `Payload` into `T`, verifying its type tag.
    pub fn get_typed_payload<T: DeserializeOwned>(
        &self,
        hash_or_prefix: &str,
        expected_type: &str,
    ) -> Result<T> {
        let obj = self.get_raw_object(hash_or_prefix)?;
        if obj.object_type != expected_type {
            bail!(
                "CAS type mismatch for {}: expected '{}', found '{}'",
                obj.hash,
                expected_type,
                obj.object_type
            );
        }
        serde_json::from_value(obj.payload)
            .with_context(|| format!("Failed to deserialize '{}' payload", expected_type))
    }

    /// Lists all objects currently stored in CAS.
    pub fn list_all_objects(&self) -> Result<Vec<CasObject>> {
        let mut results = Vec::new();
        if !self.objects_dir().exists() {
            return Ok(results);
        }
        for shard_entry in fs::read_dir(self.objects_dir())? {
            let shard_entry = shard_entry?;
            if shard_entry.path().is_dir() {
                for obj_entry in fs::read_dir(shard_entry.path())? {
                    let obj_entry = obj_entry?;
                    if obj_entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        let raw = fs::read_to_string(obj_entry.path())?;
                        let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
                        if let Ok(obj) = serde_json::from_str::<CasObject>(content) {
                            results.push(obj);
                        }
                    }
                }
            }
        }
        results.sort_by(|a, b| a.hash.cmp(&b.hash));
        Ok(results)
    }

    /// Verifies the BLAKE3 cryptographic integrity of every object in `.clibm/objects`.
    pub fn fsck(&self) -> Result<FsckReport> {
        let objects = self.list_all_objects()?;
        let mut verified = 0usize;
        let mut corrupted = Vec::new();
        let mut legacy = Vec::new();

        for obj in &objects {
            let (recomputed, _) =
                compute_object_hash(&obj.object_type, &obj.version, &obj.payload)?;
            if recomputed == obj.hash {
                verified += 1;
            } else if legacy_plugin_hash(&obj.object_type, &obj.version, &obj.payload).as_deref()
                == Some(obj.hash.as_str())
            {
                legacy.push(obj.hash.clone());
            } else {
                corrupted.push(obj.hash.clone());
            }
        }

        Ok(FsckReport {
            total_objects: objects.len(),
            verified_objects: verified,
            corrupted_hashes: corrupted,
            legacy_hashes: legacy,
        })
    }

    /// Counts object files in `.clibm/objects` without parsing them.
    pub fn count_objects(&self) -> usize {
        let Ok(shards) = fs::read_dir(self.objects_dir()) else {
            return 0;
        };
        shards
            .flatten()
            .filter(|s| s.path().is_dir())
            .filter_map(|s| fs::read_dir(s.path()).ok())
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
                    .count()
            })
            .sum()
    }

    // --- Reference & Branch Management ---

    pub fn current_branch(&self) -> Result<Option<String>> {
        let head = fs::read_to_string(self.head_file())
            .context("Failed to read .clibm/HEAD")?;
        let trimmed = head.trim();
        if let Some(ref_path) = trimmed.strip_prefix("ref: refs/heads/") {
            Ok(Some(ref_path.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn resolve_head_commit(&self) -> Result<Option<String>> {
        let head = fs::read_to_string(self.head_file())
            .context("Failed to read .clibm/HEAD")?;
        let trimmed = head.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            self.get_branch_commit(branch)
        } else if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    pub fn get_branch_commit(&self, branch: &str) -> Result<Option<String>> {
        if validate_ref_name(branch).is_err() {
            // Never turn an arbitrary string (e.g. "../../x") into a path under refs/heads.
            return Ok(None);
        }
        let branch_file = self.refs_heads_dir().join(branch);
        if !branch_file.exists() {
            return Ok(None);
        }
        let hash = fs::read_to_string(branch_file)?.trim().to_string();
        if hash.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hash))
        }
    }

    pub fn set_branch_commit(&self, branch: &str, commit_hash: &str) -> Result<()> {
        validate_ref_name(branch)?;
        let branch_file = self.refs_heads_dir().join(branch);
        if let Some(parent) = branch_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(branch_file, format!("{}\n", commit_hash))?;
        Ok(())
    }

    /// A note for repositories made before CLIBM dropped branches: HEAD detached on an
    /// old commit, HEAD on a branch other than `main`, or extra branch refs. `None` for a
    /// normal repository (HEAD -> refs/heads/main and no other refs).
    pub fn history_note(&self) -> Result<Option<String>> {
        let current = self.current_branch()?;
        let others: Vec<String> = self
            .list_branches()?
            .into_iter()
            .map(|(name, _)| name)
            .filter(|name| Some(name) != current.as_ref())
            .collect();
        let head = match current.as_deref() {
            Some("main") => None,
            Some(other) => Some(format!("HEAD is on the old branch '{}'", other)),
            None => Some("HEAD is on an old commit (detached)".to_string()),
        };
        if head.is_none() && others.is_empty() {
            return Ok(None);
        }
        let mut parts: Vec<String> = head.into_iter().collect();
        if !others.is_empty() {
            parts.push(format!("unused branch refs remain: {}", others.join(", ")));
        }
        Ok(Some(format!(
            "{}. CLIBM no longer uses branches: new commits continue from HEAD, and the other refs are ignored (nothing was deleted).",
            parts.join("; ")
        )))
    }

    pub fn set_detached_head(&self, commit_hash: &str) -> Result<()> {
        fs::write(self.head_file(), format!("{}\n", commit_hash))?;
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<(String, String)>> {
        let mut branches = Vec::new();
        if !self.refs_heads_dir().exists() {
            return Ok(branches);
        }
        for entry in fs::read_dir(self.refs_heads_dir())? {
            let entry = entry?;
            if entry.path().is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let hash = fs::read_to_string(entry.path())?.trim().to_string();
                branches.push((name, hash));
            }
        }
        branches.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(branches)
    }

    pub fn get_staged_tree_hash(&self) -> Result<Option<String>> {
        if !self.stage_file().exists() {
            return Ok(None);
        }
        let h = fs::read_to_string(self.stage_file())?.trim().to_string();
        if h.is_empty() {
            Ok(None)
        } else {
            Ok(Some(h))
        }
    }

    pub fn set_staged_tree_hash(&self, tree_hash: &str) -> Result<()> {
        fs::write(self.stage_file(), format!("{}\n", tree_hash))?;
        Ok(())
    }

    /// Resolves a reference to a commit hash: `HEAD`, `HEAD~N` (N commits before HEAD),
    /// `main` (or another ref name), or a full/short commit hash.
    pub fn resolve_ref(&self, reference: &str) -> Result<String> {
        let reference = reference.trim();
        let upper = reference.to_ascii_uppercase();
        if upper == "HEAD" {
            return self
                .resolve_head_commit()?
                .ok_or_else(|| anyhow!("HEAD does not point to any commit yet"));
        }
        if let Some(n) = upper.strip_prefix("HEAD~") {
            let steps: usize = if n.is_empty() {
                1
            } else {
                n.parse()
                    .map_err(|_| anyhow!("Invalid reference '{}': use HEAD~N, e.g. HEAD~2", reference))?
            };
            let mut hash = self
                .resolve_head_commit()?
                .ok_or_else(|| anyhow!("HEAD does not point to any commit yet"))?;
            for done in 0..steps {
                let commit: CommitPayload = self.get_typed_payload(&hash, OBJ_COMMIT)?;
                hash = commit.parent_hashes.first().cloned().ok_or_else(|| {
                    anyhow!(
                        "'{}' goes past the first commit: the history has only {} commit(s) before HEAD",
                        reference,
                        done
                    )
                })?;
            }
            return Ok(hash);
        }
        if let Some(commit_hash) = self.get_branch_commit(reference)? {
            return Ok(commit_hash);
        }
        let hash = self.resolve_hash_prefix(reference)?;
        // Only commits are valid here (not task or board objects).
        self.get_typed_payload::<CommitPayload>(&hash, OBJ_COMMIT)
            .map_err(|_| anyhow!("'{}' is not a commit (use a hash from `clibm log`)", reference))?;
        Ok(hash)
    }
}
