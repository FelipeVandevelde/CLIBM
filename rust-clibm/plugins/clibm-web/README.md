# clibm-web — local web dashboard

A single-binary local web server with an embedded Kanban dashboard for a CLIBM project.
It reads and writes the project through the CLIBM core library, so the dashboard follows the
same rules as the CLI: dependencies, WIP limits, plugin hooks and automatic commits.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.ps1)
```

From a clone: `cd plugins/clibm-web && ./install.sh` (or `.\install.ps1`), or
`clibm plugin install web`. The plugin is built from the whole Cargo workspace because it
uses the core library.

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/uninstall.ps1)
```

Or `clibm plugin uninstall web`.

## Usage

```bash
cd ~/my-project
clibm plugin run web                   # http://127.0.0.1:7878
clibm plugin run web -- --port 8080    # or: clibm-web --port 8080
```

Open the printed address in your browser. The dashboard shows the active project's board,
lets you add and move tasks, commit, and run `fsck`. A refused move shows CLIBM's reason.

## Security

The server is meant for your own browser only:

- it listens on `127.0.0.1` and sends **no CORS headers**;
- requests whose `Host` is not `127.0.0.1:<port>` / `localhost:<port>` are refused (stops
  DNS-rebinding attacks), and so are requests with a foreign `Origin` (stops other websites);
- every change needs the **session token** that is generated at each start and embedded in
  the page (`X-Clibm-Token` header).

## HTTP endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | The dashboard |
| `GET` | `/api/state` | Branch, HEAD, uncommitted changes, active project, board and its tasks |
| `GET` | `/api/fsck` | Integrity check of every stored object |
| `POST` | `/api/task` | `{"title", "priority"?, "column_id"?}` — create a task |
| `POST` | `/api/move` | `{"task_id", "column_id", "force"?}` — move a task (`task_id` accepts `101`, `APP-101` or a hash prefix) |
| `POST` | `/api/commit` | `{"message"}` — commit the working tree |

Refused writes answer `409` with `{"ok": false, "error": "..."}`; missing token or foreign
origin answer `403`.

For AI agents, use the MCP server (`clibm-mcp`) instead.
