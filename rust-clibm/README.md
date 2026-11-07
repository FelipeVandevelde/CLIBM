# CLIBM — CLI-Based Manager (v0.1.0) in Rust

A lightweight, high-performance, local-first CLI project management system written in **Rust**. Designed for native execution on **Linux** and **Windows** with zero external database dependencies.

## Key Architectural Highlights

1. **Interface-Agnostic Core (`clibm_core`)**: Complete separation between the storage/domain engine (`src/cas.rs`, `src/engine.rs`, `src/versioning.rs`, `src/index.rs`) and the `clap` CLI interface (`src/cli.rs`, `src/main.rs`).
2. **Content-Addressable Storage (CAS) with BLAKE3**:
   - All entities (`workspace`, `project`, `board`, `task`, `tree`, `commit`) are serialized into deterministic **RFC 8785 Canonical JSON** (`src/canonical.rs`) and hashed with **BLAKE3**.
   - Every stored file in `.clibm/objects/<shard>/<hash>.json` follows the immutable 4-field envelope: `Type`, `Version`, `Hash`, `Payload`.
3. **Git-Style Versioning**:
   - Supports `commit`, `log`, `branch`, `checkout`, structural `diff`, `merge`, and per-task history (`task history <ID>`).
4. **Computed Indices & DAG Validation**:
   - Maintains `.clibm/index/computed.json` for fast filtering by column, priority, tag, and assignee, and prevents circular task dependencies via DFS cycle detection.
5. **Standalone Folder-Scoped Plugins (`CLIBM_PLUGIN_CONTRACT_V1`)**:
   - Built-in modules (`dep-audit`, `export-md`, `workflow-lint`) plus standalone binary plugins (`plugins/clibm-tui` and `plugins/clibm-web`) that are built/installed strictly from within their own folders and auto-registered for execution inside any ongoing `.clibm` project.

---

## Building & Installing Core CLI (Linux & Windows)

### Prerequisites
- Rust toolchain (`rustc` & `cargo` 1.75+): https://rustup.rs

### Quick Installation (Direct from GitHub)
```powershell
# Windows PowerShell (interactive one-liner via raw GitHub script)
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.ps1 | iex

# Or download and run with parameters:
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.ps1" -OutFile install.ps1
.\install.ps1 -Yes
```

```bash
# Linux / macOS (interactive or non-interactive via raw GitHub script)
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash -s -- -y
```

### Local Installation (From Cloned Repository)
```bash
# Linux / macOS (interactive wizard; registers install dir on user PATH)
./install.sh

# Non-interactive install without modifying user PATH:
./install.sh -y --no-modify-path

# Windows PowerShell (interactive wizard; registers install dir on user PATH)
.\install.ps1

# Non-interactive Windows install without modifying user PATH:
.\install.ps1 -Yes -NoModifyPath

# Or via Cargo directly:
cargo install --path .
```

> **Note:** Open a new terminal (or reload your shell profile on Linux/macOS) after installation so the updated user `PATH` takes effect.

### User `PATH` Registration & Reference-Counted Ledger
- **User Scope Only:** Core and standalone plugin installers only update the **user `PATH`** (never system/machine `PATH`, so administrator privileges are not required):
  - **Windows (`install.ps1`):** Reads `HKCU:\Environment` `Path` with `DoNotExpandEnvironmentNames` and writes back as `REG_EXPAND_SZ` (`ExpandString`), preserving unexpanded `%USERPROFILE%` / `%SystemRoot%` segments and broadcasting `WM_SETTINGCHANGE`.
  - **Linux / macOS (`install.sh`):** Appends an idempotent `# >>> clibm PATH: <dir> >>>` ... `# <<< clibm PATH: <dir> <<<` block to existing user shell profiles (`~/.bashrc`, `~/.zshrc`, `~/.profile`, creating `~/.profile` if none exist).
- **Shared Helper & Standalone Fallback:** Canonical helper scripts live in `scripts/env-path.ps1` and `scripts/env-path.sh` and are installed to `~/.clibm/lib/`. Standalone plugin scripts verify `~/.clibm/lib/env-path.*` (checking `# clibm-env-path-version: 1`) and recreate or upgrade it from an embedded copy if missing, allowing plugins to be installed independently while warning if CLIBM core is not yet installed.
- **Ownership Ledger (`~/.clibm/path-ledger`):** Tracks `<owner>\t<dir>\t<added_by_clibm>`. Uninstalling a plugin removes a `PATH` entry only when zero owners remain **and** `added_by_clibm` is `1` (pre-existing directories such as `~/.cargo/bin` are never removed).
- **Opt-Out:** Pass `-NoModifyPath` (PowerShell) or `--no-modify-path` (Bash) to skip `PATH` and ledger changes.
- **Manual Cleanup:** If you delete binaries manually without running `uninstall.ps1` / `uninstall.sh`, remove the `# >>> clibm PATH: ... >>>` block from `~/.bashrc` / `~/.zshrc` / `~/.profile` (or remove the folder from `HKCU:\Environment` `Path` on Windows) and delete `~/.clibm/path-ledger`.

### Quick Start Commands (Ongoing Project)
```bash
# 1. Initialize repository in your project folder
mkdir ~/my-project && cd ~/my-project
clibm init --workspace "engineering" --code "CLIBM" --name "CLIBM Core"

# 2. Add tasks with priorities, tags, and dependencies
clibm task add "Implement BLAKE3 canonical serializer" -p critical -c done --tags storage,crypto
clibm task add "Build Git-style tree diff engine" -p high -c in-progress --depends-on CLIBM-101

# 3. Inspect the Kanban board
clibm board show

# 4. Commit snapshot, create branch, and run structural diff
clibm commit -m "Complete CAS and diff engine tasks"
clibm fsck
```

---

## Building, Installing & Running Plugins in an Ongoing Project

Plugins are standalone Cargo crates located in their own directories (`plugins/clibm-tui` and `plugins/clibm-web`). Each plugin has its own `README.md` and install scripts and must be built or installed from within its own folder.

### 1. Install a Plugin (Direct from GitHub or From Local Folder)
```powershell
# Windows PowerShell — Quick install directly from GitHub:
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.ps1 | iex
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.ps1 | iex
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/install.ps1 | iex
```

```bash
# Linux / macOS — Quick install directly from GitHub:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.sh | bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.sh | bash

# Or install from its local folder inside a cloned repository:
cd plugins/clibm-tui
./install.sh          # On Windows: .\install.ps1 (or: clibm plugin install .)

cd ../clibm-web
./install.sh          # On Windows: .\install.ps1 (or: clibm plugin install .)
```

### 2. Run the Installed Plugin Inside Any Ongoing Project
Once installed, navigate to **any ongoing CLIBM project folder** (containing `.clibm/`) and launch the plugin either via `clibm plugin run` or directly as a binary:

```bash
cd ~/my-project

# Verify installed & registered plugins:
clibm plugin list

# Launch the interactive Ratatui TUI inside your ongoing project:
clibm plugin run clibm-tui    # (or shorthand: clibm plugin run tui)
# Or invoke the installed binary directly:
clibm-tui

# Launch the self-hosted web workbench on http://127.0.0.1:7878 for your ongoing project:
clibm plugin run clibm-web -- --port 7878   # (or shorthand: clibm plugin run web)
# Or invoke the installed binary directly:
clibm-web --port 7878
```

### 3. Create a New Custom Plugin from the Official Template
Use the built-in CLI scaffolder or copy `plugins/plugin-template` to create a new standalone plugin crate:

```bash
# Scaffold a new standalone plugin crate in plugins/clibm-sprint-velocity:
clibm plugin new sprint-velocity --hooks pre-commit,on-task-move

# Build and install from its folder:
cd plugins/clibm-sprint-velocity
./install.sh

# Run inside any ongoing project:
clibm plugin run clibm-sprint-velocity
```

---

## Updating CLIBM from GitHub

You can update CLIBM and its standalone plugins directly from the configured GitHub repository (`github_repo_url` in `.clibm/config.json`):

```bash
# Check for upstream updates without installing:
clibm update --check

# Pull and install the latest core binary (and rebuild plugins):
clibm update --plugins

# Or use the standalone update scripts:
./update.sh --plugins
# Windows PowerShell:
.\update.ps1 -Plugins
```

---

## Uninstalling CLIBM

By default, uninstalling removes the `clibm` core binary and all installed `clibm-*` plugin binaries (including custom `install_path` directories recorded in `~/.clibm/install-config.json` and `~/.clibm/path-ledger`), cleans up any user `PATH` entries added by CLIBM alongside `~/.clibm/lib/`, and **preserves** your `.clibm/` project repositories. Pass `--purge` (`-Purge` on PowerShell) to also delete local `.clibm/` repository data and `~/.clibm/install-config.json`:

```bash
# Safe uninstall (keeps .clibm/ repository data):
clibm uninstall
# Or via script:
./uninstall.sh
.\uninstall.ps1

# Full uninstall + delete local .clibm/ repository data:
clibm uninstall --purge
# Or via script:
./uninstall.sh --purge
.\uninstall.ps1 -Purge
```

