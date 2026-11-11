# CLIBM Self-Hosted Model Context Protocol Server (`clibm-mcp` v0.2.0)

Modular, self-hosted Model Context Protocol (MCP) server plugin (`2024-11-05` specification) allowing AI agents and IDEs (Claude Desktop, Cursor, Windsurf, custom LLM orchestrators) to inspect and mutate `.clibm` repositories deterministically.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-mcp/uninstall.ps1)
```

---

## Architecture Highlights

- **Shared MCP Adapter**: Fully decoupled core engine providing `ToolRegistry`, `ResourceRegistry`, `PromptRegistry`, `PermissionManager`, and `RequestDispatcher`.
- **Compile-Time Transport Gating**:
  - `transport-stdio`: Lightweight STDIO JSON-RPC 2.0 without HTTP dependencies or network sockets.
  - `transport-http`: Local HTTP JSON-RPC 2.0 server with CORS support.
  - Both enabled: True concurrent dual-transport execution sharing a single `Arc<McpAdapter>`.
- **Permission & Safety Model**: Enforces workspace boundary containment and supports `--read-only` policy.
- **Strict Protocol Compliance**: Fully adheres to the Model Context Protocol (`2024-11-05`) and JSON-RPC 2.0 (§4.1) by silently consuming notification requests (such as `notifications/initialized`), ensuring compatibility with strict MCP client SDKs without connection drops.

## Cargo Feature Flags

| Feature | Description | Dependency Impact |
|---|---|---|
| `transport-stdio` | Enables stdio JSON-RPC 2.0 communication | Zero networking crates |
| `transport-http` | Enables local HTTP JSON-RPC 2.0 server | Includes `tiny_http` |
| `default` | Enables both transports | Full dual capability |

```bash
# Build STDIO only (minimal size, zero networking)
cargo build --release --no-default-features --features transport-stdio

# Build HTTP only
cargo build --release --no-default-features --features transport-http

# Build both (default)
cargo build --release --features "transport-stdio,transport-http"
```

## Interactive Transport Configuration During Installation

When you run `./install.sh` (or `.\install.ps1` on Windows) inside `plugins/clibm-mcp`, the installer compiles only the chosen transport:

```powershell
# Windows PowerShell:
cd plugins\clibm-mcp
.\install.ps1
# Or non-interactively:
.\install.ps1 -Transport stdio
```

```bash
# Linux / macOS:
cd plugins/clibm-mcp
./install.sh
# Or non-interactively:
./install.sh --transport stdio
```

## Running Inside an Ongoing CLIBM Project

```bash
cd ~/my-clibm-project

# Run in stdio mode (Claude Desktop / Cursor MCP config):
clibm plugin run mcp -- --stdio

# Run self-hosted HTTP server on custom port:
clibm plugin run mcp -- --http --port 7879

# Run in read-only mode (prevents mutating tool calls):
clibm plugin run mcp -- --stdio --read-only

# Run both transports simultaneously (HTTP runs in background thread):
clibm plugin run mcp -- --both
```

## AI Agent MCP Configuration JSON

To connect `clibm-mcp` with AI agents and coding assistants (Google Antigravity, Claude Desktop, Cursor, Windsurf, Claude Code, Cline, Roo Code, or custom LLM orchestrators), configure your client's MCP configuration JSON file using the templates below.

### 1. Google Antigravity / Gemini CLI (`mcp_config.json`)

Antigravity supports both local STDIO executables and Streamable HTTP endpoints. Configure globally in `~/.gemini/config/mcp_config.json` or in your workspace config:

#### STDIO Transport
```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm-mcp",
      "args": ["--stdio", "--root", "/absolute/path/to/your/clibm/project"]
    }
  }
}
```
*(On Windows, you can specify `"clibm-mcp.exe"` or the full path `"%USERPROFILE%\\.clibm-bin\\clibm-mcp.exe"` if not yet in your system `PATH`.)*

#### HTTP Transport (Self-Hosted Daemon)
```json
{
  "mcpServers": {
    "clibm-http": {
      "serverUrl": "http://127.0.0.1:7879"
    }
  }
}
```

---

### 2. Claude Desktop

Add `clibm` under the `mcpServers` object in your Claude Desktop configuration file:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

#### Recommended: Standard CLI Invocation (Workspace-Bound)
```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm",
      "args": ["plugin", "run", "mcp", "--", "--stdio"],
      "env": {
        "CLIBM_PROJECT_ROOT": "/absolute/path/to/your/clibm/project"
      }
    }
  }
}
```

#### Alternative: Direct Standalone Binary Invocation (`clibm-mcp`)
If you prefer calling the installed binary directly without the `clibm plugin run` runner:
```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm-mcp",
      "args": ["--stdio", "--root", "/absolute/path/to/your/clibm/project"]
    }
  }
}
```
*(On Windows, you can specify `"clibm-mcp.exe"` or the full path `"%USERPROFILE%\\.clibm-bin\\clibm-mcp.exe"` if not yet on system `PATH`.)*

---

### 3. Cursor IDE (`.cursor/mcp.json`)

Cursor natively supports repository-level MCP configurations. Create or edit `.cursor/mcp.json` at the root of your project:

```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm",
      "args": ["plugin", "run", "mcp", "--", "--stdio"]
    }
  }
}
```

Or configure globally in Cursor via **Settings** -> **Features** -> **MCP** -> **+ Add New MCP Server**:
- **Name**: `clibm`
- **Type**: `command`
- **Command**: `clibm plugin run mcp -- --stdio`

---

### 4. Windsurf (`~/.codeium/windsurf/mcp_config.json`)

Windsurf reads MCP servers from `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm",
      "args": ["plugin", "run", "mcp", "--", "--stdio"],
      "env": {
        "CLIBM_PROJECT_ROOT": "${workspaceFolder}"
      }
    }
  }
}
```

---

### 5. VS Code Extensions (Cline, Roo Code, Claude Dev)

In Cline / Roo Code MCP Settings (`cline_mcp_settings.json`):

```json
{
  "mcpServers": {
    "clibm": {
      "command": "clibm",
      "args": ["plugin", "run", "mcp", "--", "--stdio"],
      "disabled": false,
      "autoApprove": [
        "clibm_status",
        "clibm_task_list"
      ]
    }
  }
}
```

---

### 6. Safe Read-Only Policy for Autonomous Agents

To allow autonomous agents or multi-agent pipelines to inspect tasks, query Kanban columns, and analyze commit trees without permission to mutate your backlog or switch workspaces, append `--read-only` to the server arguments:

```json
{
  "mcpServers": {
    "clibm-readonly": {
      "command": "clibm",
      "args": ["plugin", "run", "mcp", "--", "--stdio", "--read-only"]
    }
  }
}
```

When `--read-only` is active:
- Query tools (`clibm_status`, `clibm_task_list`, non-mutating plugin tools) execute normally.
- Mutating tools (`clibm_task_add`, `clibm_board_move`, `clibm_workspace_switch`, mutating plugin tools) are automatically rejected with an access-denied error code.

---

### 7. Local HTTP Server Configuration (Multi-Agent & Web Clients)

When running `clibm-mcp` with the HTTP transport (`clibm plugin run mcp -- --http --port 7879`):

#### Standard / Antigravity Streamable HTTP:
```json
{
  "mcpServers": {
    "clibm-http": {
      "serverUrl": "http://127.0.0.1:7879"
    }
  }
}
```

#### Custom Transport Clients:
```json
{
  "mcpServers": {
    "clibm-http": {
      "url": "http://127.0.0.1:7879",
      "transport": "http"
    }
  }
}
```

- **`GET /`**: Returns server capability discovery metadata, version info, and links to documentation.
- **`POST /`**: Full JSON-RPC 2.0 endpoint for agent method dispatching (`initialize`, `tools/list`, `tools/call`, `resources/list`, etc.).


## Exposed Capabilities

### 1. Tools (`tools/list`, `tools/call`)

#### Native Tools
- `clibm_status` — Inspect active workspace, branch, HEAD commit, and BLAKE3 CAS stats.
- `clibm_task_list` — Query tasks in the active workspace or across `--all-workspaces`.
- `clibm_task_add` — Create an immutable task in the active workspace (auto-committed via BLAKE3).
- `clibm_board_move` — Transition a task across Kanban columns with DAG dependency validation.
- `clibm_workspace_switch` — Switch active workspace by ID/name or cycle alphabetically (`A->Z`).

#### Dynamic Plugin Tools (Registry-Driven)
`clibm-mcp` automatically reads the local repository's `.clibm/plugins/registry.json` (and user-level registry) to expose tools declared by enabled, installed plugins:
- Namespaced identifiers (e.g. `dep_audit.audit_dependencies`, `export_md.export_project`).
- Validates input arguments against the declared JSON Schema before invocation.
- Dispatches execution to the owning plugin via `clibm plugin run <plugin_name> [args]`.
- Live reloading: detects modifications to `registry.json` and updates the active tool set on the fly without restarting the MCP server (supported by advertising `"tools": { "listChanged": true }`).

### Declaring Tools in `registry.json`

Add a `tools` array (or `mcp: { "tools": [...] }`) to any plugin's entry in `.clibm/plugins/registry.json`:

```json
[
  {
    "name": "clibm-dep-audit",
    "version": "0.1.0",
    "description": "Dependency graph auditor",
    "enabled": true,
    "executable": "~/.clibm-bin/clibm-dep-audit.exe",
    "tools": [
      {
        "name": "audit_dependencies",
        "description": "Audit task DAG for circular dependencies and blockers",
        "input_schema": {
          "type": "object",
          "properties": {
            "format": { "type": "string", "enum": ["text", "json"] }
          }
        },
        "is_mutating": false,
        "execution_ref": "audit"
      }
    ]
  }
]
```

### 2. Live CAS Resources (`resources/list`, `resources/read`)
- `clibm://tree/staged` — Current staged BLAKE3 CAS objects and manifest trees.
- `clibm://config` — Repository configuration, branch heads, and active workspace metadata.
- `clibm://board/active` — Active ASCII Kanban board visualization.

### 3. Agent Prompts (`prompts/list`, `prompts/get`)
- `clibm_task_planning` — Requirements decomposition and task breakdown prompt template.
- `clibm_commit_summary` — Conventional BLAKE3 commit message generation prompt template.
