# CLIBM Kanban WIP & Policy Linter (`clibm-workflow-lint`)

Standalone Rust plugin extracted from the pure CLIBM core. Validates Kanban column WIP limits, unblocked dependencies, and assignee rules.

## 1. Build & Install (Direct from GitHub or Local Folder)

```powershell
# Windows PowerShell — Quick install directly from GitHub:
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/install.ps1 | iex
```

```bash
# Linux / macOS — Quick install directly from GitHub:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-workflow-lint/install.sh | bash

# Or from local plugin folder:
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
