# clibm-dep-audit — dependency audit

Audits the task dependency graph of the **current** project state:

- circular dependency chains;
- dependencies on tasks that no longer exist;
- **blockers**: open tasks that other open tasks are waiting on;
- the **critical path**: the longest chain of open dependencies.

It answers `ok: false` (exit code 1 when run by hand) when it finds a cycle or a missing
dependency.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/install.ps1)
```

From a clone: `cd plugins/clibm-dep-audit && ./install.sh` (or `.\install.ps1`), or
`clibm plugin install dep-audit`.

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/uninstall.ps1)
```

Or `clibm plugin uninstall dep-audit`.

## Usage

```bash
cd ~/my-project
clibm plugin run dep-audit            # active project
clibm plugin run dep-audit -- --all   # every project
clibm-dep-audit --all                 # same, run directly
```

Example:

```text
[clibm-dep-audit] Dependency audit of App (APP): 4 task(s), tree 93c3181606fd
  Cycles: none
  Blockers (open tasks that others wait on):
    APP-101 "Base" [todo] blocks 2: APP-102, APP-104
  Critical path (3 open tasks): APP-101 -> APP-102 -> APP-103
```

**Hook:** `on-task-depend` (after `clibm task depend`), informational.
**MCP tool:** `dep_audit__audit_dependencies` (read-only, optional `all`).
