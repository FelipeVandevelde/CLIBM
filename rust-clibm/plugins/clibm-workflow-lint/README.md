# CLIBM Kanban WIP & Policy Linter (`clibm-workflow-lint`)

Standalone Rust plugin extracted from the pure CLIBM core. Validates Kanban column WIP limits, unblocked dependencies, and assignee rules.

## 1. Build & Install (From This Plugin Folder)

```bash
cd plugins/clibm-workflow-lint
./install.sh
# Windows PowerShell:
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project
clibm plugin run workflow-lint
```
