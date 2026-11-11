# clibm-tui — Ratatui Terminal User Interface Plugin (v0.1.0)

`clibm-tui` is a standalone **Ratatui + Crossterm** Terminal User Interface plugin for **CLIBM**. It connects directly to any ongoing `.clibm/` Content-Addressable Storage (CAS) repository and provides an interactive split-pane Kanban board, canonical task inspector, commit creator, and `BLAKE3` integrity verifier inside your terminal.

---

## 1. Quick Lifecycle (Direct from GitHub)

### Quick Install
```powershell
# Windows PowerShell:
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.ps1)
```
```bash
# Linux / macOS / WSL2:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.sh | bash
```

### Quick Uninstall
```powershell
# Windows PowerShell:
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/uninstall.ps1)
```
```bash
# Linux / macOS / WSL2:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/uninstall.sh | bash
```

---

## 2. Local Installation & Build (From Plugin Folder)
```bash
cd plugins/clibm-tui   # (or cd clibm-tui if extracted from clibm-tui-v0.1.0.zip)
./install.sh
```

### Local Installation (Windows PowerShell)
```powershell
cd plugins\clibm-tui   # (or cd clibm-tui if extracted from clibm-tui-v0.1.0.zip)
.\install.ps1
```

### Or via the CLIBM CLI / Cargo inside this folder
```bash
cd plugins/clibm-tui
clibm plugin install .
# or manually:
cargo build --release
./target/release/clibm-tui --register
```

What installation does:
1. Compiles the optimized release binary (`target/release/clibm-tui`).
2. Installs the binary into `~/.local/bin/clibm-tui` (Linux) or `%USERPROFILE%\.clibm-bin\clibm-tui.exe` (Windows).
3. Registers `clibm-tui` in your global CLIBM plugin registry (`~/.clibm/plugins/registry.json`) so it is immediately available across all ongoing CLIBM projects.

---

## 2. Running Inside an Ongoing CLIBM Project

Once installed, navigate to **any ongoing CLIBM project folder** (where `.clibm/` exists via `clibm init`) and launch the plugin using either method:

### Method A: Via `clibm plugin run`
```bash
cd ~/my-ongoing-project
clibm plugin run tui
# or full plugin name:
clibm plugin run clibm-tui
```

### Method B: Direct Binary Execution
```bash
cd ~/my-ongoing-project
clibm-tui
```

`clibm-tui` automatically discovers the `.clibm/` repository in your current (or parent) project directory and loads all active Kanban columns, tasks, branches, and `BLAKE3` objects.

---

## 3. Keyboard Shortcuts

| Key | Action |
|---|---|
| `h` / `Left` | Focus previous Kanban column |
| `l` / `Right` | Focus next Kanban column |
| `j` / `Down` | Select next task card in active column |
| `k` / `Up` | Select previous task card in active column |
| `m` | Move selected task to the next Kanban column (writes new immutable `BLAKE3` task & tree object) |
| `c` | Create an immutable Git-style commit snapshot of the current staged tree |
| `f` | Run `BLAKE3` cryptographic `fsck` verification across all `.clibm/objects/*` |
| `q` / `Esc` | Exit `clibm-tui` and return to terminal |
