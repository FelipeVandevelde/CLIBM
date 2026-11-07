# CLIBM DAG Critical Path & Cycle Auditor (`clibm-dep-audit`)

Standalone Rust plugin extracted from the pure CLIBM core. Audits `.clibm/index/computed.json` and task dependency graphs to detect circular dependencies and highlight critical-path blockers.

## 1. Build & Install (From This Plugin Folder)

```bash
cd plugins/clibm-dep-audit
./install.sh
# Windows PowerShell:
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project
clibm plugin run dep-audit
# Or invoke the binary directly:
clibm-dep-audit
```
