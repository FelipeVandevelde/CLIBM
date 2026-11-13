//! Small helper for writing CLIBM plugins in Rust on top of `clibm_core`.
//!
//! A plugin is started by CLIBM in two ways:
//! - `clibm plugin run <name> [-- args]` or a lifecycle hook: a `CLIBM_PLUGIN_CONTRACT_V1`
//!   JSON request arrives on stdin and a `{"ok": bool, "output": "..."}` reply is
//!   expected on stdout;
//! - by hand from a terminal: no request; the plugin prints plain text and its exit code
//!   tells success.
//!
//! [`PluginRun`] hides that difference.
//!
//! ```no_run
//! use clibm_core::plugin_sdk::PluginRun;
//! let run = PluginRun::from_env();
//! let engine = run.engine();
//! // ... inspect the project ...
//! run.finish(true, "All good".to_string());
//! ```

use crate::engine::ClibmEngine;
use crate::plugin::{PluginInvocationRequest, PluginInvocationResponse};
use serde_json::Value;
use std::env;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

pub struct PluginRun {
    /// The JSON request, when CLIBM started the plugin.
    pub request: Option<PluginInvocationRequest>,
    /// Command-line arguments (without the program name).
    pub args: Vec<String>,
    /// Root of the project (`CLIBM_PROJECT_ROOT`, else the current directory).
    pub project_root: PathBuf,
}

impl PluginRun {
    /// Reads the arguments, the project root and (when stdin is not a terminal) the
    /// JSON request.
    pub fn from_env() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        let project_root = env::var("CLIBM_PROJECT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let mut input = String::new();
        if !io::stdin().is_terminal() {
            let _ = io::stdin().read_to_string(&mut input);
        }
        let request = serde_json::from_str::<PluginInvocationRequest>(input.trim()).ok();
        Self {
            request,
            args,
            project_root,
        }
    }

    /// `manual-run` for `clibm plugin run` and direct runs, else the hook event name.
    pub fn event(&self) -> &str {
        self.request
            .as_ref()
            .map(|r| r.event.as_str())
            .unwrap_or("manual-run")
    }

    /// Event details sent with a lifecycle hook (`Value::Null` otherwise).
    pub fn context(&self) -> &Value {
        static NULL: Value = Value::Null;
        self.request.as_ref().map(|r| &r.context).unwrap_or(&NULL)
    }

    /// Opens the project. Hooks are disabled: a plugin must never trigger other plugins.
    pub fn engine(&self) -> ClibmEngine {
        ClibmEngine::open(&self.project_root).without_hooks()
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|a| a == flag)
    }

    pub fn value_of(&self, flag: &str) -> Option<&str> {
        self.args
            .iter()
            .position(|a| a == flag)
            .and_then(|i| self.args.get(i + 1))
            .map(String::as_str)
    }

    /// Replies and exits: a JSON reply when CLIBM started the plugin, otherwise plain
    /// text with exit code 0 (ok) or 1.
    pub fn finish(&self, ok: bool, output: String) -> ! {
        if self.request.is_some() {
            let reply = PluginInvocationResponse { ok, output };
            println!(
                "{}",
                serde_json::to_string(&reply).unwrap_or_else(|_| "{\"ok\":false,\"output\":\"\"}".into())
            );
            std::process::exit(0);
        }
        println!("{}", output);
        std::process::exit(if ok { 0 } else { 1 });
    }
}
