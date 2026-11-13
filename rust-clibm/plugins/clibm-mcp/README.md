# clibm-mcp — Model Context Protocol server for CLIBM

`clibm-mcp` lets AI agents and coding assistants (Claude Desktop, Claude Code, Cursor,
Windsurf, Cline, custom agents) read and manage a CLIBM project through the
[Model Context Protocol](https://modelcontextprotocol.io) (`2024-11-05`).

It is a thin, safe layer over the `clibm` CLI: every tool runs a `clibm` command in the
project, so the agent follows exactly the same rules as you do (dependencies, WIP limits,
plugin hooks, automatic commits).

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/install.ps1)
```

The installer asks which transport to build (default **stdio**, the one AI clients use).
Non-interactive: `./install.sh -y --transport both` / `.\install.ps1 -Yes -Transport both`.
From a clone: `cd plugins/clibm-mcp && ./install.sh` (or `.\install.ps1`), or
`clibm plugin install mcp --transport stdio`.

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/uninstall.ps1)
```

Or simply `clibm plugin uninstall mcp`. A running `clibm-mcp` (for example one started by an
AI client) is stopped first, so the binary can always be removed.

---

## Command line

```text
clibm-mcp [OPTIONS]
clibm plugin run mcp -- [OPTIONS]

  --stdio          JSON-RPC 2.0 over stdin/stdout (default)
  --http           Local HTTP endpoint: POST http://127.0.0.1:<port>/mcp
  --both           stdio in the foreground and HTTP in the background
  -p, --port <P>   Port for HTTP (default 7879)
  --root <PATH>    Project folder containing .clibm (default: current folder or CLIBM_PROJECT_ROOT)
  --read-only      Refuse tools that change the project
  --print-config   Print the MCP client configuration (.mcp.json snippet)
  -V, --version    Print the version
  -h, --help       Print help
```

Unknown options are rejected (exit code 2). Defaults can also be set in
`~/.clibm/mcp-config.json` (`transport`, `http_port`, `read_only`), which the installer writes.

### Cargo features

| Feature | Transport | Default |
|---|---|---|
| `transport-stdio` | stdin/stdout | yes |
| `transport-http` | local HTTP (`tiny_http`) | yes |

```bash
cargo build --release -p clibm-mcp --no-default-features --features transport-stdio   # stdio only
cargo build --release -p clibm-mcp --no-default-features --features transport-http    # HTTP only
```

---

## AI Agent MCP Configuration JSON

`clibm-mcp --print-config` prints the minimal configuration:

```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm-mcp",
      "args": ["--stdio"]
    }
  }
}
```

The server works on the project in its **current folder** (or a parent folder). When your
client does not start it inside the project, add `"--root", "/absolute/path/to/project"` to
`args`. If `clibm-mcp` is not on the `PATH` the client sees, use the full path
(`~/.local/bin/clibm-mcp` or `%USERPROFILE%\.clibm-bin\clibm-mcp.exe`).

| Client | Where the configuration goes |
|---|---|
| Claude Code | `.mcp.json` at the project root, or `claude mcp add clibm -- clibm-mcp --stdio` |
| Claude Desktop | macOS `~/Library/Application Support/Claude/claude_desktop_config.json`, Windows `%APPDATA%\Claude\claude_desktop_config.json`, Linux `~/.config/Claude/claude_desktop_config.json` (add `--root`) |
| Cursor | `.cursor/mcp.json` in the project |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` (add `--root`) |
| VS Code (Cline, Roo Code) | the extension's MCP settings file (add `--root`) |

### Safe read-only mode for autonomous agents

```json
{
  "mcpServers": {
    "clibm-readonly": {
      "command": "clibm-mcp",
      "args": ["--stdio", "--read-only", "--root", "/absolute/path/to/project"]
    }
  }
}
```

Tools that change the project (`clibm_task_add`, `clibm_board_move`,
`clibm_workspace_switch`, and plugin tools declared as mutating) are refused.

### Local HTTP endpoint

Start it with `clibm-mcp --http` (or `--both`), then point HTTP-capable clients at
`http://127.0.0.1:7879/mcp`:

```json
{
  "mcpServers": {
    "clibm-http": { "type": "http", "url": "http://127.0.0.1:7879/mcp" }
  }
}
```

The endpoint is a minimal *Streamable HTTP* server: `POST /mcp` takes one JSON-RPC message
or a batch and answers with `application/json`; notifications get `202 Accepted`; `GET`
answers `405` (no server-sent event stream). It binds `127.0.0.1` only, sends no CORS
headers, and refuses requests whose `Host` or `Origin` is not the local server, so other
websites cannot reach it from your browser.

---

## Exposed capabilities

### Tools

| Tool | Changes the project | What it does |
|---|---|---|
| `clibm_status` | no | Branch, HEAD, uncommitted changes, active workspace/project, counts per column (JSON) |
| `clibm_task_list` | no | Tasks as JSON; filters `all_workspaces`, `priority`, `column`, `tag` |
| `clibm_task_show` | no | One task as JSON; `task` = `APP-101`, `101` or a hash prefix |
| `clibm_task_add` | yes | Create a task (`title`, optional `description`, `priority`, `column`, `assignee`, `depends_on`) |
| `clibm_board_move` | yes | Move a task to a column; refusals (dependency, WIP limit, plugin veto) explain why |
| `clibm_workspace_switch` | yes | Switch workspace by ID/name, or cycle alphabetically |

When a tool fails (for example a move blocked by a dependency) the result is returned with
`isError: true` and CLIBM's message, so the agent can read it and react.

#### Plugin tools

Installed plugins can declare MCP tools in their manifest (`<plugin> --manifest`, stored in
`~/.clibm/plugins/registry.json`). They appear as `<plugin>__<tool>`, for example
`dep_audit__audit_dependencies`, `export_md__export_project`, `workflow_lint__lint_workflow`.

- Only plugins that are **installed** (binary present) and **enabled** expose tools.
- Names always carry the plugin prefix, so a plugin can never replace a built-in tool or
  another plugin's tool.
- Arguments are validated against the declared `input_schema` and passed to
  `clibm plugin run <plugin> -- <execution_ref> --<arg> <value>`.

A tool declaration in a plugin manifest:

```json
{
  "name": "audit_dependencies",
  "description": "Audit the task dependency graph",
  "input_schema": { "type": "object", "properties": { "all": { "type": "boolean" } } },
  "is_mutating": false,
  "execution_ref": "audit"
}
```

### Resources

| URI | Content |
|---|---|
| `clibm://tree/staged` | The current (staged) tree object: IDs mapped to object hashes |
| `clibm://config` | `.clibm/config.json` |
| `clibm://board/active` | The active board as text (`clibm board show`) |

### Prompts

`clibm_task_planning` (break a goal into tasks) and `clibm_commit_summary` (summarize recent
history).
