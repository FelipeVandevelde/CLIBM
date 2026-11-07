# CLIBM Markdown & Sprint Report Exporter (`clibm-export-md`)

Standalone Rust plugin extracted from the pure CLIBM core. Transforms `.clibm/` task trees into portable Markdown sprint reports.

## 1. Build & Install (Direct from GitHub or Local Folder)

```powershell
# Windows PowerShell — Quick install directly from GitHub:
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/install.ps1 | iex
```

```bash
# Linux / macOS — Quick install directly from GitHub:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/install.sh | bash

# Or from local plugin folder:
cd plugins/clibm-export-md
./install.sh
# Windows PowerShell (local):
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project
clibm plugin run export-md > SPRINT_REPORT.md
```
