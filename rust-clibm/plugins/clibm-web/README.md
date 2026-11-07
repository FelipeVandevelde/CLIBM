# clibm-web — Single-Binary Self-Hosted Web Server Plugin (v0.1.0)

`clibm-web` is a standalone local HTTP server plugin for **CLIBM** written in Rust (`tiny_http`). It embeds a zero-dependency monochromatic web workbench (`src/ui.html`) directly into the compiled binary via `include_str!("ui.html")` and exposes local REST JSON endpoints over any ongoing `.clibm/` Content-Addressable Storage (CAS) project.

---

## 1. Folder-Scoped Build & Installation

`clibm-web` is a standalone Cargo crate and **must be built or installed from within its own `clibm-web` directory**:

### Linux
```bash
cd plugins/clibm-web   # (or cd clibm-web if extracted from clibm-web-v0.1.0.zip)
./install.sh
```

### Windows (PowerShell)
```powershell
cd plugins\clibm-web   # (or cd clibm-web if extracted from clibm-web-v0.1.0.zip)
.\install.ps1
```

### Or via the CLIBM CLI / Cargo inside this folder
```bash
cd plugins/clibm-web
clibm plugin install .
# or manually:
cargo build --release
./target/release/clibm-web --register
```

What installation does:
1. Compiles the single-binary release executable (`target/release/clibm-web`) with `ui.html` embedded inside the binary.
2. Installs the binary into `~/.local/bin/clibm-web` (Linux) or `%USERPROFILE%\.clibm-bin\clibm-web.exe` (Windows).
3. Registers `clibm-web` in your global CLIBM plugin registry (`~/.clibm/plugins/registry.json`) so it is immediately available across all ongoing CLIBM projects.

---

## 2. Running Inside an Ongoing CLIBM Project

Once installed, navigate to **any ongoing CLIBM project folder** (where `.clibm/` exists via `clibm init`) and launch the plugin using either method:

### Method A: Via `clibm plugin run`
```bash
cd ~/my-ongoing-project
clibm plugin run web
# or with a custom port:
clibm plugin run clibm-web -- --port 8080
```

### Method B: Direct Binary Execution
```bash
cd ~/my-ongoing-project
clibm-web --port 7878
```

Then open **`http://127.0.0.1:7878`** in any browser to manage tasks, Kanban columns, and `BLAKE3` commits in real time.

---

## 3. Local HTTP Endpoints (`127.0.0.1:7878`)

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/` | Serves the embedded monochromatic HTML/JS web dashboard (`src/ui.html`) |
| `GET` | `/api/state` | Returns active branch, staged tree hash, Kanban columns, tasks, commit history, and `BLAKE3` `fsck` status |
| `POST` | `/api/task` | Creates a new canonical task object in `.clibm/objects/*` and updates `STAGE_TREE` |
| `POST` | `/api/move` | Moves a task card to a target Kanban column and writes a new immutable `BLAKE3` task & tree object |
| `POST` | `/api/commit` | Commits the current `STAGE_TREE` snapshot to the active branch ref |
