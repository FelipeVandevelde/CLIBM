# clibm-export-md — Markdown report

Writes a Markdown report of the **current** project state: one table per board, tasks in
column order, with ID, title, priority, column, assignee, due date, dependencies and tags.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/install.ps1)
```

From a clone: `cd plugins/clibm-export-md && ./install.sh` (or `.\install.ps1`), or
`clibm plugin install export-md`.

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/uninstall.ps1)
```

Or `clibm plugin uninstall export-md`.

## Usage

```bash
cd ~/my-project
clibm plugin run export-md                          # print the active project's report
clibm plugin run export-md -- --all                 # every project
clibm plugin run export-md -- --output sprint.md    # write it to a file
```

**Hook:** `on-export`. With the hook enabled, `clibm export -o bundle.json` also writes
`bundle.md` next to the bundle.
**MCP tool:** `export_md__export_project` (read-only, optional `all`).
