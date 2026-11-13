//! Shared helpers for CLIBM integration tests: an isolated temporary project with its
//! own fake home directory (`CLIBM_HOME`), so tests never touch the real `~/.clibm`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct TempProject {
    pub root: PathBuf,
    pub home: PathBuf,
    pub dir: PathBuf,
}

impl TempProject {
    pub fn new(label: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "clibm-it-{}-{}-{}-{}",
            label,
            std::process::id(),
            n,
            nanos
        ));
        let home = root.join("home");
        let dir = root.join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        Self { root, home, dir }
    }

    /// Runs the `clibm` binary in `cwd` with an isolated home directory.
    pub fn clibm_in(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_clibm"))
            .args(args)
            .current_dir(cwd)
            .env("CLIBM_HOME", &self.home)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove("CLIBM_PROJECT_ROOT")
            .env("CLIBM_NO_HOOKS", "1")
            .output()
            .expect("failed to run clibm")
    }

    pub fn clibm(&self, args: &[&str]) -> Output {
        self.clibm_in(&self.dir, args)
    }

    /// Runs `clibm` and panics with its output when it fails.
    pub fn ok(&self, args: &[&str]) -> String {
        let out = self.clibm(args);
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(
            out.status.success(),
            "clibm {:?} failed:\nstdout: {}\nstderr: {}",
            args,
            stdout,
            String::from_utf8_lossy(&out.stderr)
        );
        stdout
    }

    /// Runs `clibm` expecting failure; returns stderr + stdout.
    pub fn fails(&self, args: &[&str]) -> String {
        let out = self.clibm(args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(!out.status.success(), "clibm {:?} unexpectedly succeeded:\n{}", args, text);
        text
    }

    pub fn json(&self, args: &[&str]) -> serde_json::Value {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let out = self.ok(&full);
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("invalid JSON from {:?}: {}\n{}", args, e, out))
    }

    pub fn init(&self) {
        self.ok(&["init", "--code", "APP", "--name", "App"]);
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
