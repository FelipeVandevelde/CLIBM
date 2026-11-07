# CLIBM Self-Hosted Model Context Protocol Server (`clibm-mcp`)

Standalone Rust MCP plugin enabling AI agents and IDEs (Claude Desktop, Cursor, Windsurf, local LLM orchestrators) to inspect and mutate `.clibm` repositories deterministically over JSON-RPC 2.0.

## Interactive Transport Configuration During Installation

When you run `./install.sh` (or `.\install.ps1` on Windows) inside `plugins/clibm-mcp`, the installer prompts you to choose your preferred MCP transport mode:

1. **`both` (Default)** — Enables both **stdio JSON-RPC 2.0** and **Local HTTP + SSE Server** (`127.0.0.1:7879`)
2. **`stdio` only** — Stdio JSON-RPC 2.0 transport only
3. **`http` only** — Local HTTP + SSE JSON-RPC 2.0 server only (`127.0.0.1:7879`)

```bash
cd plugins/clibm-mcp
./install.sh
# Or non-interactively:
./install.sh --transport both|stdio|http
```

## Running Inside an Ongoing CLIBM Project

```bash
cd ~/my-clibm-project

# Run in stdio mode (for Claude Desktop / Cursor MCP config):
clibm plugin run mcp -- --stdio

# Run self-hosted HTTP + SSE server on port 7879:
clibm plugin run mcp -- --http --port 7879
```

## Exposed MCP Tools

- `clibm_status` — Inspect active workspace, branch, HEAD commit, and BLAKE3 CAS stats
- `clibm_task_list` — Query tasks in the active workspace or across `--all-workspaces`
- `clibm_task_add` — Create a new task (automatically committed via BLAKE3)
- `clibm_board_move` — Transition a task across Kanban columns with DAG dependency validation
- `clibm_workspace_switch` — Switch active workspace by ID/name or cycle alphabetically (`A->Z`)
