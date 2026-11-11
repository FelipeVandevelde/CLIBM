//! Standalone CLIBM AI Teaching & Documentation Plugin (`clibm-docs`)
//!
//! Teaches AI agents and LLMs (Claude Code, Cursor, Windsurf, Copilot, custom agents)
//! how to operate CLIBM deterministically. Outputs a token-efficient AI instruction
//! manual to stdout and optionally writes `AGENTS.md`, `CLAUDE.md`, `.cursorrules`,
//! and `.mcp.json` into the active project root.

use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let project_root = env::var("CLIBM_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let write_files = args.iter().any(|a| a == "--write" || a == "-w");

    let guide = build_ai_teaching_guide();

    if write_files {
        fs::write(project_root.join("AGENTS.md"), &guide)?;
        fs::write(project_root.join("CLAUDE.md"), &guide)?;
        fs::write(project_root.join(".cursorrules"), &guide)?;

        let mcp_config = serde_json::json!({
            "mcpServers": {
                "clibm": {
                    "command": "clibm",
                    "args": ["plugin", "run", "mcp", "--", "--stdio"]
                }
            }
        });
        fs::write(
            project_root.join(".mcp.json"),
            serde_json::to_string_pretty(&mcp_config)? + "\n",
        )?;

        println!(
            "[clibm-docs] Generated AGENTS.md, CLAUDE.md, .cursorrules, and .mcp.json in {}",
            project_root.display()
        );
    } else {
        println!("{}", guide);
    }

    Ok(())
}

fn build_ai_teaching_guide() -> String {
    r#"# CLIBM v0.1.0 — AI Agent Instruction Manual (`AGENTS.md` / `CLAUDE.md`)

You are operating inside a **CLIBM (CLI-Based Manager)** project repository stored in `.clibm/`.
CLIBM is a local-first project manager backed by **RFC 8785 Canonical JSON** and **BLAKE3 Content-Addressable Storage (CAS)**.

## 1. Core Architectural Rules for AI Agents
- **Automatic Per-Modification Commits (`auto_commit = true`)**: Every task creation, edit, move, transfer, comment, or workspace switch automatically creates an atomic BLAKE3 commit (`auto(task): ...`, `auto(workspace): ...`). You do NOT need to run `clibm commit` after each change unless `auto_commit` is disabled.
- **Alphabetical Workspace Cycling**: Running `clibm workspace switch` without an ID or name automatically advances to the next workspace in alphabetical order (`A -> Z`).
- **Pure Core & Standalone Plugins**: The core engine has 0 built-in plugins. All extensions (`clibm-mcp`, `clibm-docs`, `clibm-tui`, `clibm-web`, `clibm-dep-audit`, `clibm-export-md`, `clibm-workflow-lint`) are standalone binaries installed from `./plugins/<name>`.

## 2. Essential CLI Commands for AI Agents

```bash
# Inspect repository state, active workspace, branch, and BLAKE3 CAS stats:
clibm status

# List all workspaces (sorted A->Z) and switch workspaces:
clibm workspace list
clibm workspace switch <ID_OR_NAME>
clibm workspace switch                 # Cycles to next workspace in alphabetical order (A->Z)

# List tasks (active workspace or across all workspaces):
clibm task list
clibm task list --all-workspaces

# Create, move, comment, link dependencies, or transfer tasks across workspaces:
clibm task add "<title>" -p <low|medium|high|critical> -c <backlog|todo|in-progress|review|done>
clibm board move <TASK_ID> <COLUMN_ID>
clibm task depend <TASK_ID> <UPSTREAM_TASK_ID>
clibm task transfer <TASK_ID> <TARGET_WORKSPACE_ID> [--copy]

# Manage repository configuration (.clibm/config.json):
clibm config show
clibm config set auto_commit true|false
```

## 3. Using the Self-Hosted MCP Server (`clibm-mcp`)
If MCP is enabled in your agent runtime, prefer invoking the structured MCP tools (`clibm_status`, `clibm_task_list`, `clibm_task_add`, `clibm_board_move`, `clibm_workspace_switch`) exposed by `clibm-mcp`.

For the complete MCP configuration guide for Claude Desktop, Cursor, Windsurf, and agent pipelines, see:
https://github.com/FelipeVandevelde/CLIBM/blob/main/rust-clibm/plugins/clibm-mcp/README.md#ai-agent-mcp-configuration-json

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
"#
    .to_string()
}
