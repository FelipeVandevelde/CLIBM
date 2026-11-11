# CLIBM Kanban WIP & Policy Linter (`clibm-workflow-lint`)

Standalone Rust plugin extracted from the pure CLIBM core. Validates Kanban column WIP limits, unblocked dependencies, and assignee rules.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/uninstall.ps1)
```

---

## 1. Local Build & Install (From Plugin Folder)

```bash
cd plugins/clibm-workflow-lint
./install.sh
# Windows PowerShell (local):
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project
clibm plugin run workflow-lint
```

## 3. Model Context Protocol (MCP) Integration

When `clibm-mcp` is running, this plugin's tool is automatically discovered from `registry.json` and exposed to AI agents as:
- **Tool Identifier**: `workflow_lint.lint_workflow`
- **Description**: Validates Kanban WIP limits, unblocked dependencies, and assignee constraints.
- **Execution Mode**: Read-only (`is_mutating: false`).
