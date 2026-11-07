# CLIBM Markdown & Sprint Report Exporter (`clibm-export-md`)

Standalone Rust plugin extracted from the pure CLIBM core. Transforms `.clibm/` task trees into portable Markdown sprint reports.

## 1. Build & Install (From This Plugin Folder)

```bash
cd plugins/clibm-export-md
./install.sh
# Windows PowerShell:
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project
clibm plugin run export-md > SPRINT_REPORT.md
```
