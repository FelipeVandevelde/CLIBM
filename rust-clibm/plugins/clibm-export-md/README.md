# CLIBM Markdown & Sprint Report Exporter (`clibm-export-md`)

Standalone Rust plugin extracted from the pure CLIBM core. Transforms `.clibm/` task trees into portable Markdown sprint reports.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-export-md/uninstall.ps1)
```

---

## 1. Local Build & Install (From Plugin Folder)

```bash
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

## 3. Model Context Protocol (MCP) Integration

When `clibm-mcp` is running, this plugin's tool is automatically discovered from `registry.json` and exposed to AI agents as:
- **Tool Identifier**: `export_md.export_project`
- **Description**: Generates a portable Markdown sprint report from the project's CAS task snapshot.
- **Execution Mode**: Read-only (`is_mutating: false`).
