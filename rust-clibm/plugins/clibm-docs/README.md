# CLIBM AI Documentation & Context Teacher (`clibm-docs`)

Standalone Rust plugin that teaches AI agents and coding assistants (Claude Code, Cursor, Windsurf, GitHub Copilot) how to interact with CLIBM repositories accurately.

## Capabilities

1. **Stdout AI Context Manual**: Prints a structured, token-efficient instruction manual covering CLIBM's BLAKE3 CAS architecture, automatic commit rules, multi-workspace coordination, and exact CLI grammar.
2. **Project Instruction File Generator (`--write`)**: Automatically generates `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, and `.mcp.json` tailored to your active `.clibm` repository.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-docs/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-docs/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-docs/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-docs/uninstall.ps1)
```

---

## 1. Local Build & Install (From Plugin Folder)

```bash
cd plugins/clibm-docs
./install.sh
# Windows PowerShell (local):
.\install.ps1
```

## 2. Run Inside Any Ongoing CLIBM Project

```bash
cd ~/my-clibm-project

# Print the AI Agent Instruction Manual to stdout:
clibm plugin run docs

# Generate AGENTS.md, CLAUDE.md, and .cursorrules in the project root:
clibm plugin run docs -- --write
```
