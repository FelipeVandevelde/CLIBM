# CLIBM DAG Critical Path & Cycle Auditor (`clibm-dep-audit`)

Standalone Rust plugin extracted from the pure CLIBM core. Audits `.clibm/index/computed.json` and task dependency graphs to detect circular dependencies and highlight critical-path blockers.

## 1. Build & Install (Direct from GitHub or Local Folder)

```powershell
# Windows PowerShell — Quick install directly from GitHub:
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/install.ps1 | iex
```

```bash
# Linux / macOS — Quick install directly from GitHub:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/install.sh | bash

# Or from local plugin folder:
cd plugins/clibm-dep-audit
./install.sh
# Windows PowerShell (local):
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project
clibm plugin run dep-audit
# Or invoke the binary directly:
clibm-dep-audit
```
