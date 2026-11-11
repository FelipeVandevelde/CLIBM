# CLIBM DAG Critical Path & Cycle Auditor (`clibm-dep-audit`)

Standalone Rust plugin extracted from the pure CLIBM core. Audits `.clibm/index/computed.json` and task dependency graphs to detect circular dependencies and highlight critical-path blockers.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-dep-audit/uninstall.ps1)
```

---

## 1. Local Build & Install (From Plugin Folder)

```bash
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

## 3. Model Context Protocol (MCP) Integration

When `clibm-mcp` is running, this plugin's tool is automatically discovered from `registry.json` and exposed to AI agents as:
- **Tool Identifier**: `dep_audit.audit_dependencies`
- **Description**: Audits the task dependency DAG in `.clibm/index/computed.json` to detect circular chains and blockers.
- **Execution Mode**: Read-only (`is_mutating: false`).
