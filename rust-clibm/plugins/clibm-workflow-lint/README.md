# clibm-workflow-lint — Kanban workflow checks

Checks the **current** project state against the Kanban workflow.

| Kind | Check |
|---|---|
| violation | A column holds more tasks than its WIP limit |
| violation | A task is in progress without an assignee while its board requires one |
| violation | A completed task depends on a task that is still open |
| warning | A task is in progress without an assignee |
| warning | A task is overdue (`due` before today, not done) |
| warning | A task depends on a task that does not exist |

Violations make it answer `ok: false` (exit code 1 when run by hand).

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/install.ps1)
```

From a clone: `cd plugins/clibm-workflow-lint && ./install.sh` (or `.\install.ps1`), or
`clibm plugin install workflow-lint`.

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/uninstall.ps1)
```

Or `clibm plugin uninstall workflow-lint`.

## Usage

```bash
cd ~/my-project
clibm plugin run workflow-lint            # active project
clibm plugin run workflow-lint -- --all   # every project
```

### Hooks

- **`post-commit` (default):** after every change, violations are reported as a warning.
  The core already enforces these rules on moves, so violations only appear after someone
  used `--force`.
- **`pre-move` (optional):** install with `--hooks pre-move` (or `-Hooks pre-move`) to have
  it refuse a move that would break a rule. Moves made with `--force` are let through.

**MCP tool:** `workflow_lint__lint_workflow` (read-only, optional `all`).
