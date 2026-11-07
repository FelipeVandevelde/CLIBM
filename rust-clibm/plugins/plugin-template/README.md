# CLIBM Plugin Starter Template (`clibm-plugin-template`)

Official starter template for creating standalone **CLIBM v0.1.0** plugins in Rust.

## Key Capabilities

1. **Folder-Scoped Standalone Compilation**: Compiles strictly from within its own plugin folder without modifying the core `clibm` Cargo manifest.
2. **Ongoing-Project Execution**: Reads `.clibm/HEAD`, `.clibm/config.json`, and `.clibm/objects/` from the active project directory (`CLIBM_PROJECT_ROOT` or current working directory).
3. **`CLIBM_PLUGIN_CONTRACT_V1` Protocol**: Parses JSON invocation payloads over `stdin` and emits structured JSON responses over `stdout`.

## Quick Start: Creating a New Plugin

### Option A: Using the CLI Generator (Recommended)

From any initialized CLIBM project directory:

```bash
clibm plugin new my-custom-plugin --hooks pre-commit,on-task-move
cd plugins/clibm-my-custom-plugin
./install.sh
```

### Option B: Copying `plugins/plugin-template` Manually

```bash
cp -r plugins/plugin-template plugins/clibm-my-plugin
cd plugins/clibm-my-plugin
# Edit Cargo.toml and src/main.rs
./install.sh
```

### Option C: Direct Install from GitHub Raw Script

```powershell
# Windows PowerShell:
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/plugin-template/install.ps1 | iex
```

```bash
# Linux / macOS:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/plugin-template/install.sh | bash
```

## Running Your Plugin in an Ongoing Project

Once installed via `./install.sh` (or `.\install.ps1` on Windows) or `clibm plugin install .`, run your plugin inside any CLIBM project:

```bash
clibm plugin run clibm-plugin-template
# or invoke the installed binary directly:
clibm-plugin-template
```

## Uninstalling the Plugin

```bash
./uninstall.sh
```
