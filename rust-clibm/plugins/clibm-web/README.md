# clibm-web — Single-Binary Self-Hosted Web Server & Unified AI MCP Plugin (v0.1.0)

`clibm-web` is a standalone local HTTP server plugin for **CLIBM** written in Rust (`tiny_http`). It embeds a zero-dependency monochromatic web workbench (`src/ui.html`) directly into the compiled binary via `include_str!("ui.html")` and exposes both local REST JSON endpoints and a unified **Model Context Protocol (MCP)** server over any ongoing `.clibm/` Content-Addressable Storage (CAS) project.

By unifying the Web Dashboard and the MCP HTTP server on a single port (`127.0.0.1:7878`), `clibm-web` cuts memory and socket usage in half while sharing a single compilation cache through the Cargo workspace.

---

## 1. Quick Lifecycle (Direct from GitHub)

### Quick Install
```powershell
# Windows PowerShell:
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.ps1)
```
```bash
# Linux / macOS / WSL2:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.sh | bash
```

### Quick Uninstall
```powershell
# Windows PowerShell:
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/uninstall.ps1)
```
```bash
# Linux / macOS / WSL2:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/uninstall.sh | bash
```

---

## 2. Local Installation & Build (From Plugin Folder)
```bash
cd plugins/clibm-web   # (or cd clibm-web if extracted from clibm-web-v0.1.0.zip)
./install.sh
```

### Local Installation (Windows PowerShell)
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
1. Compiles the single-binary release executable with `ui.html` and the embedded MCP adapter inside the binary.
2. Installs the lightweight (~1.3 MB) binary into `~/.local/bin/clibm-web` (Linux) or `%USERPROFILE%\.clibm-bin\clibm-web.exe` (Windows).
3. **Space Optimization:** Automatically cleans intermediate build artifacts (`target/`) post-installation so only the small executable stays on disk.
4. Registers `clibm-web` in your global CLIBM plugin registry (`~/.clibm/plugins/registry.json`) with hooks `http-bridge`, `on-state-query`, `mcp-http-sse`, and `on-ai-tool-call`.

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

- **Browser Dashboard:** Open **`http://127.0.0.1:7878`** in any browser to manage tasks, Kanban columns, and `BLAKE3` commits in real time.
- **AI Agent MCP Server:** Point AI tools (Cursor, Claude Desktop, Antigravity) to **`http://127.0.0.1:7878/mcp`** (or POST `http://127.0.0.1:7878/`) to execute tools and query tasks over the unified port.

---

## 3. Local HTTP Endpoints (`127.0.0.1:7878`)

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/` | Serves the embedded monochromatic HTML/JS web dashboard (`src/ui.html`) |
| `GET` | `/mcp` | Returns MCP server capabilities JSON (protocol version, tools, resources, prompts) |
| `POST` | `/mcp` or `/` | Dispatches JSON-RPC 2.0 requests to the built-in CLIBM MCP adapter |
| `OPTIONS` | `*` | Universal CORS preflight headers allowing browser extensions and IDE webviews |
| `GET` | `/api/state` | Returns active branch, staged tree hash, Kanban columns, tasks, commit history, and `BLAKE3` `fsck` status |
| `POST` | `/api/task` | Creates a new canonical task object in `.clibm/objects/*` and updates `STAGE_TREE` |
| `POST` | `/api/move` | Moves a task card to a target Kanban column and writes a new immutable `BLAKE3` task & tree object |
| `POST` | `/api/commit` | Commits the current `STAGE_TREE` snapshot to the active branch ref |
