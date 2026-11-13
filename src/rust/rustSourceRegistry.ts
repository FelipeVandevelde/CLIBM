/**
 * Registry of all Rust source files in `/rust-clibm/` (100% Pure Core + CLI + Standalone Plugins in `plugins/*`)
 * imported as raw strings so users can inspect, copy, or download:
 * 1. Pure Core-Only Package (`clibm-core-v0.1.0.zip`) — strictly `clibm_core` + `clibm` CLI with 0 plugins
 * 2. Individual Standalone Plugin Packages (`clibm-mcp-v0.1.0.zip`, `clibm-docs-v0.1.0.zip`, `clibm-dep-audit-v0.1.0.zip`, `clibm-export-md-v0.1.0.zip`, `clibm-workflow-lint-v0.1.0.zip`, `clibm-tui-v0.1.0.zip`, `clibm-web-v0.1.0.zip`, `clibm-plugin-template-v0.1.0.zip`)
 * 3. Complete Bundle (`clibm-v0.1.0-workspace.zip`) — Pure Core CLI + all standalone plugin folders in `plugins/`
 *
 * All installation, update, and uninstallation files (`install.sh`, `install.ps1`, `update.sh`, `update.ps1`,
 * `uninstall.sh`, `uninstall.ps1`) and ZIP archives are UTF-8-BOM (`EF BB BF`) encoded for maximum compatibility.
 */

import cargoToml from '../../rust-clibm/Cargo.toml?raw';
import readmeMd from '../../rust-clibm/README.md?raw';
import installSh from '../../rust-clibm/install.sh?raw';
import installPs1 from '../../rust-clibm/install.ps1?raw';
import mainRs from '../../rust-clibm/src/main.rs?raw';
import libRs from '../../rust-clibm/src/lib.rs?raw';
import cliRs from '../../rust-clibm/src/cli.rs?raw';
import canonicalRs from '../../rust-clibm/src/canonical.rs?raw';
import casRs from '../../rust-clibm/src/cas.rs?raw';
import modelsRs from '../../rust-clibm/src/models.rs?raw';
import indexRs from '../../rust-clibm/src/index.rs?raw';
import versioningRs from '../../rust-clibm/src/versioning.rs?raw';
import pluginRs from '../../rust-clibm/src/plugin.rs?raw';
import engineRs from '../../rust-clibm/src/engine.rs?raw';
import uninstallSh from '../../rust-clibm/uninstall.sh?raw';
import uninstallPs1 from '../../rust-clibm/uninstall.ps1?raw';
import updateSh from '../../rust-clibm/update.sh?raw';
import updatePs1 from '../../rust-clibm/update.ps1?raw';
import envPathSh from '../../rust-clibm/scripts/env-path.sh?raw';
import envPathPs1 from '../../rust-clibm/scripts/env-path.ps1?raw';
import syncEnvPathPs1 from '../../rust-clibm/scripts/sync-env-path.ps1?raw';

// Standalone Plugins in `plugins/`
import mcpCargoToml from '../../rust-clibm/plugins/clibm-mcp/Cargo.toml?raw';
import mcpMainRs from '../../rust-clibm/plugins/clibm-mcp/src/main.rs?raw';
import mcpConfigRs from '../../rust-clibm/plugins/clibm-mcp/src/config.rs?raw';
import mcpErrorRs from '../../rust-clibm/plugins/clibm-mcp/src/error.rs?raw';
import mcpProtocolModRs from '../../rust-clibm/plugins/clibm-mcp/src/protocol/mod.rs?raw';
import mcpProtocolTypesRs from '../../rust-clibm/plugins/clibm-mcp/src/protocol/types.rs?raw';
import mcpProtocolSchemaRs from '../../rust-clibm/plugins/clibm-mcp/src/protocol/schema.rs?raw';
import mcpBackendModRs from '../../rust-clibm/plugins/clibm-mcp/src/backend/mod.rs?raw';
import mcpBackendBridgeRs from '../../rust-clibm/plugins/clibm-mcp/src/backend/bridge.rs?raw';
import mcpAdapterModRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/mod.rs?raw';
import mcpAdapterDiscoveryRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/discovery.rs?raw';
import mcpAdapterDispatcherRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/dispatcher.rs?raw';
import mcpAdapterPermissionsRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/permissions.rs?raw';
import mcpAdapterToolsRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/tools.rs?raw';
import mcpAdapterValidatorRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/validator.rs?raw';
import mcpAdapterResourcesRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/resources.rs?raw';
import mcpAdapterPromptsRs from '../../rust-clibm/plugins/clibm-mcp/src/adapter/prompts.rs?raw';
import mcpTransportModRs from '../../rust-clibm/plugins/clibm-mcp/src/transport/mod.rs?raw';
import mcpTransportStdioRs from '../../rust-clibm/plugins/clibm-mcp/src/transport/stdio.rs?raw';
import mcpTransportHttpRs from '../../rust-clibm/plugins/clibm-mcp/src/transport/http.rs?raw';
import mcpReadmeMd from '../../rust-clibm/plugins/clibm-mcp/README.md?raw';
import mcpInstallSh from '../../rust-clibm/plugins/clibm-mcp/install.sh?raw';
import mcpInstallPs1 from '../../rust-clibm/plugins/clibm-mcp/install.ps1?raw';
import mcpUninstallSh from '../../rust-clibm/plugins/clibm-mcp/uninstall.sh?raw';
import mcpUninstallPs1 from '../../rust-clibm/plugins/clibm-mcp/uninstall.ps1?raw';

import docsCargoToml from '../../rust-clibm/plugins/clibm-docs/Cargo.toml?raw';
import docsMainRs from '../../rust-clibm/plugins/clibm-docs/src/main.rs?raw';
import docsReadmeMd from '../../rust-clibm/plugins/clibm-docs/README.md?raw';
import docsInstallSh from '../../rust-clibm/plugins/clibm-docs/install.sh?raw';
import docsInstallPs1 from '../../rust-clibm/plugins/clibm-docs/install.ps1?raw';
import docsUninstallSh from '../../rust-clibm/plugins/clibm-docs/uninstall.sh?raw';
import docsUninstallPs1 from '../../rust-clibm/plugins/clibm-docs/uninstall.ps1?raw';

import depAuditCargoToml from '../../rust-clibm/plugins/clibm-dep-audit/Cargo.toml?raw';
import depAuditMainRs from '../../rust-clibm/plugins/clibm-dep-audit/src/main.rs?raw';
import depAuditReadmeMd from '../../rust-clibm/plugins/clibm-dep-audit/README.md?raw';
import depAuditInstallSh from '../../rust-clibm/plugins/clibm-dep-audit/install.sh?raw';
import depAuditInstallPs1 from '../../rust-clibm/plugins/clibm-dep-audit/install.ps1?raw';
import depAuditUninstallSh from '../../rust-clibm/plugins/clibm-dep-audit/uninstall.sh?raw';
import depAuditUninstallPs1 from '../../rust-clibm/plugins/clibm-dep-audit/uninstall.ps1?raw';

import exportMdCargoToml from '../../rust-clibm/plugins/clibm-export-md/Cargo.toml?raw';
import exportMdMainRs from '../../rust-clibm/plugins/clibm-export-md/src/main.rs?raw';
import exportMdReadmeMd from '../../rust-clibm/plugins/clibm-export-md/README.md?raw';
import exportMdInstallSh from '../../rust-clibm/plugins/clibm-export-md/install.sh?raw';
import exportMdInstallPs1 from '../../rust-clibm/plugins/clibm-export-md/install.ps1?raw';
import exportMdUninstallSh from '../../rust-clibm/plugins/clibm-export-md/uninstall.sh?raw';
import exportMdUninstallPs1 from '../../rust-clibm/plugins/clibm-export-md/uninstall.ps1?raw';

import workflowLintCargoToml from '../../rust-clibm/plugins/clibm-workflow-lint/Cargo.toml?raw';
import workflowLintMainRs from '../../rust-clibm/plugins/clibm-workflow-lint/src/main.rs?raw';
import workflowLintReadmeMd from '../../rust-clibm/plugins/clibm-workflow-lint/README.md?raw';
import workflowLintInstallSh from '../../rust-clibm/plugins/clibm-workflow-lint/install.sh?raw';
import workflowLintInstallPs1 from '../../rust-clibm/plugins/clibm-workflow-lint/install.ps1?raw';
import workflowLintUninstallSh from '../../rust-clibm/plugins/clibm-workflow-lint/uninstall.sh?raw';
import workflowLintUninstallPs1 from '../../rust-clibm/plugins/clibm-workflow-lint/uninstall.ps1?raw';

import tuiCargoToml from '../../rust-clibm/plugins/clibm-tui/Cargo.toml?raw';
import tuiMainRs from '../../rust-clibm/plugins/clibm-tui/src/main.rs?raw';
import tuiReadmeMd from '../../rust-clibm/plugins/clibm-tui/README.md?raw';
import tuiInstallSh from '../../rust-clibm/plugins/clibm-tui/install.sh?raw';
import tuiInstallPs1 from '../../rust-clibm/plugins/clibm-tui/install.ps1?raw';
import tuiUninstallSh from '../../rust-clibm/plugins/clibm-tui/uninstall.sh?raw';
import tuiUninstallPs1 from '../../rust-clibm/plugins/clibm-tui/uninstall.ps1?raw';

import webCargoToml from '../../rust-clibm/plugins/clibm-web/Cargo.toml?raw';
import webMainRs from '../../rust-clibm/plugins/clibm-web/src/main.rs?raw';
import webUiHtml from '../../rust-clibm/plugins/clibm-web/src/ui.html?raw';
import webReadmeMd from '../../rust-clibm/plugins/clibm-web/README.md?raw';
import webInstallSh from '../../rust-clibm/plugins/clibm-web/install.sh?raw';
import webInstallPs1 from '../../rust-clibm/plugins/clibm-web/install.ps1?raw';
import webUninstallSh from '../../rust-clibm/plugins/clibm-web/uninstall.sh?raw';
import webUninstallPs1 from '../../rust-clibm/plugins/clibm-web/uninstall.ps1?raw';

import tplCargoToml from '../../rust-clibm/plugins/plugin-template/Cargo.toml?raw';
import tplMainRs from '../../rust-clibm/plugins/plugin-template/src/main.rs?raw';
import tplReadmeMd from '../../rust-clibm/plugins/plugin-template/README.md?raw';
import tplInstallSh from '../../rust-clibm/plugins/plugin-template/install.sh?raw';
import tplInstallPs1 from '../../rust-clibm/plugins/plugin-template/install.ps1?raw';
import tplUninstallSh from '../../rust-clibm/plugins/plugin-template/uninstall.sh?raw';
import tplUninstallPs1 from '../../rust-clibm/plugins/plugin-template/uninstall.ps1?raw';

import {
  buildZipArchive,
  createUtf8BomBlob,
  ensureUtf8Bom,
  triggerBlobDownload,
  ZipFileEntry,
} from './zipBuilder';

export interface RustProjectFile {
  relativePath: string;
  zipPath: string;
  category:
    | 'Manifest & Docs'
    | 'CLI & Entrypoint'
    | 'CAS & Hashing'
    | 'Core Engine & Versioning'
    | 'Plugins (TUI & Web)'
    | 'Build Scripts';
  description: string;
  language: 'rust' | 'toml' | 'markdown' | 'bash' | 'powershell' | 'html';
  content: string;
  lines: number;
  isCoreOnly: boolean;
}

export interface PluginInstallChoiceOption {
  value: string;
  label: string;
  detail: string;
}

export interface PluginInstallPromptSpec {
  flagName: string;
  ps1ParamName: string;
  promptTitle: string;
  promptDescription: string;
  defaultValue: string;
  options: PluginInstallChoiceOption[];
}

export interface DownloadablePluginPackage {
  id: string;
  name: string;
  crateName: string;
  version: string;
  category:
    | 'AI & MCP'
    | 'Interface Plugin'
    | 'Plugin Template'
    | 'Analysis & Audit'
    | 'Export & Interop'
    | 'Workflow Policy';
  summary: string;
  architectureNotes: string;
  stack: string;
  hooks: string[];
  installCommand: string;
  cliCommand: string;
  binaryInvoke: string;
  zipFilename: string;
  installPrompt: PluginInstallPromptSpec;
  files: ZipFileEntry[];
}

function countLines(str: string): number {
  return str.split('\n').length;
}

export const CORE_ONLY_CARGO_TOML = ensureUtf8Bom(cargoToml);

export const RUST_PROJECT_FILES: RustProjectFile[] = [
  {
    relativePath: 'Cargo.toml',
    zipPath: 'clibm/Cargo.toml',
    category: 'Manifest & Docs',
    description:
      'Standalone Cargo package manifest configuring 100% pure clibm_core lib and clibm CLI binary.',
    language: 'toml',
    content: ensureUtf8Bom(cargoToml),
    lines: countLines(cargoToml),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/main.rs',
    zipPath: 'clibm/src/main.rs',
    category: 'CLI & Entrypoint',
    description:
      'Cross-platform native CLI binary entrypoint executing commands, Pure Core setup, and standalone plugin execution.',
    language: 'rust',
    content: ensureUtf8Bom(mainRs),
    lines: countLines(mainRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/cli.rs',
    zipPath: 'clibm/src/cli.rs',
    category: 'CLI & Entrypoint',
    description:
      'Clap v4 command grammar including init (--with-defaults, --lang, --auto-commit, --install-path) and plugin install (--transport).',
    language: 'rust',
    content: ensureUtf8Bom(cliRs),
    lines: countLines(cliRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/lib.rs',
    zipPath: 'clibm/src/lib.rs',
    category: 'CLI & Entrypoint',
    description: 'Library crate root exposing the UI-agnostic, 100% pure clibm_core modules.',
    language: 'rust',
    content: ensureUtf8Bom(libRs),
    lines: countLines(libRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/canonical.rs',
    zipPath: 'clibm/src/canonical.rs',
    category: 'CAS & Hashing',
    description:
      'RFC 8785 Canonical JSON key-sorting serializer and BLAKE3 256-bit envelope hash calculation.',
    language: 'rust',
    content: ensureUtf8Bom(canonicalRs),
    lines: countLines(canonicalRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/cas.rs',
    zipPath: 'clibm/src/cas.rs',
    category: 'CAS & Hashing',
    description:
      'Content-Addressable Storage (.clibm/objects/<shard>/<hash>.json), upward ongoing project discovery, and fsck verifier.',
    language: 'rust',
    content: ensureUtf8Bom(casRs),
    lines: countLines(casRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/models.rs',
    zipPath: 'clibm/src/models.rs',
    category: 'Core Engine & Versioning',
    description:
      'Immutable payload structs for Workspace, Project, Task, KanbanBoard, TreeSnapshot, Commit, and ClibmConfig (install_profile, language, install_path, mcp_transport).',
    language: 'rust',
    content: ensureUtf8Bom(modelsRs),
    lines: countLines(modelsRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/engine.rs',
    zipPath: 'clibm/src/engine.rs',
    category: 'Core Engine & Versioning',
    description:
      'Headless ClibmEngine coordinating workspaces, projects, tasks, Kanban workflow rules, and portable bundles.',
    language: 'rust',
    content: ensureUtf8Bom(engineRs),
    lines: countLines(engineRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/versioning.rs',
    zipPath: 'clibm/src/versioning.rs',
    category: 'Core Engine & Versioning',
    description:
      'Git-inspired commit DAG traversal, structural diffing, task history extraction, and 3-way branch merging.',
    language: 'rust',
    content: ensureUtf8Bom(versioningRs),
    lines: countLines(versioningRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/index.rs',
    zipPath: 'clibm/src/index.rs',
    category: 'Core Engine & Versioning',
    description:
      'Computed secondary indices (.clibm/index/computed.json) and DFS circular dependency detector.',
    language: 'rust',
    content: ensureUtf8Bom(indexRs),
    lines: countLines(indexRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'src/plugin.rs',
    zipPath: 'clibm/src/plugin.rs',
    category: 'Core Engine & Versioning',
    description:
      'Pure Core plugin registry (~/.clibm/plugins/registry.json) with UTF-8-BOM scaffolding and embedded user PATH helper templates.',
    language: 'rust',
    content: ensureUtf8Bom(pluginRs),
    lines: countLines(pluginRs),
    isCoreOnly: true,
  },
  {
    relativePath: 'README.md',
    zipPath: 'clibm/README.md',
    category: 'Manifest & Docs',
    description: 'Build instructions, user PATH ledger reference, and CLI guide for running CLIBM and plugins on Linux and Windows.',
    language: 'markdown',
    content: ensureUtf8Bom(readmeMd),
    lines: countLines(readmeMd),
    isCoreOnly: true,
  },
  {
    relativePath: 'install.sh',
    zipPath: 'clibm/install.sh',
    category: 'Build Scripts',
    description:
      'UTF-8-BOM Linux/macOS interactive Core Installation Wizard (Profile, Language, Auto-Commit, Install Path, and User PATH registration with --no-modify-path).',
    language: 'bash',
    content: ensureUtf8Bom(installSh),
    lines: countLines(installSh),
    isCoreOnly: true,
  },
  {
    relativePath: 'install.ps1',
    zipPath: 'clibm/install.ps1',
    category: 'Build Scripts',
    description:
      'UTF-8-BOM Windows PowerShell interactive Core Installation Wizard (Profile, Language, Auto-Commit, Custom Path, and REG_EXPAND_SZ User PATH registration with -NoModifyPath).',
    language: 'powershell',
    content: ensureUtf8Bom(installPs1),
    lines: countLines(installPs1),
    isCoreOnly: true,
  },
  {
    relativePath: 'update.sh',
    zipPath: 'clibm/update.sh',
    category: 'Build Scripts',
    description: 'UTF-8-BOM Linux/macOS GitHub repository updater script supporting --repo, --branch, and --plugins.',
    language: 'bash',
    content: ensureUtf8Bom(updateSh),
    lines: countLines(updateSh),
    isCoreOnly: true,
  },
  {
    relativePath: 'update.ps1',
    zipPath: 'clibm/update.ps1',
    category: 'Build Scripts',
    description: 'UTF-8-BOM Windows PowerShell GitHub repository updater script supporting -Repo, -Branch, and -Plugins.',
    language: 'powershell',
    content: ensureUtf8Bom(updatePs1),
    lines: countLines(updatePs1),
    isCoreOnly: true,
  },
  {
    relativePath: 'uninstall.sh',
    zipPath: 'clibm/uninstall.sh',
    category: 'Build Scripts',
    description: 'UTF-8-BOM Linux/macOS uninstaller removing clibm/plugin binaries and CLIBM-added user PATH entries while preserving .clibm data unless --purge is passed.',
    language: 'bash',
    content: ensureUtf8Bom(uninstallSh),
    lines: countLines(uninstallSh),
    isCoreOnly: true,
  },
  {
    relativePath: 'uninstall.ps1',
    zipPath: 'clibm/uninstall.ps1',
    category: 'Build Scripts',
    description: 'UTF-8-BOM Windows PowerShell uninstaller removing clibm/plugin binaries and CLIBM-added user PATH entries while preserving .clibm data unless -Purge is passed.',
    language: 'powershell',
    content: ensureUtf8Bom(uninstallPs1),
    lines: countLines(uninstallPs1),
    isCoreOnly: true,
  },
  {
    relativePath: 'scripts/env-path.sh',
    zipPath: 'clibm/scripts/env-path.sh',
    category: 'Build Scripts',
    description: 'Shared POSIX/Bash user PATH helper (~/.clibm/lib/env-path.sh) managing profile marker blocks and ~/.clibm/path-ledger reference counting.',
    language: 'bash',
    content: ensureUtf8Bom(envPathSh),
    lines: countLines(envPathSh),
    isCoreOnly: true,
  },
  {
    relativePath: 'scripts/env-path.ps1',
    zipPath: 'clibm/scripts/env-path.ps1',
    category: 'Build Scripts',
    description: 'Shared Windows PowerShell user PATH helper (~/.clibm/lib/env-path.ps1) managing REG_EXPAND_SZ HKCU:\\Environment Path and ~/.clibm/path-ledger.',
    language: 'powershell',
    content: ensureUtf8Bom(envPathPs1),
    lines: countLines(envPathPs1),
    isCoreOnly: true,
  },
  {
    relativePath: 'scripts/sync-env-path.ps1',
    zipPath: 'clibm/scripts/sync-env-path.ps1',
    category: 'Build Scripts',
    description: 'Developer synchronization & drift-verification script (-Check) keeping embedded env-path helpers aligned across all standalone plugin scripts.',
    language: 'powershell',
    content: ensureUtf8Bom(syncEnvPathPs1),
    lines: countLines(syncEnvPathPs1),
    isCoreOnly: true,
  },

  // Standalone AI MCP Plugin (plugins/clibm-mcp)
  {
    relativePath: 'plugins/clibm-mcp/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-mcp/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the self-hosted Model Context Protocol (MCP) server plugin.',
    language: 'toml',
    content: ensureUtf8Bom(mcpCargoToml),
    lines: countLines(mcpCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/main.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description:
      'Modular MCP entry point with compile-time transport selection and dual-transport runtime.',
    language: 'rust',
    content: ensureUtf8Bom(mcpMainRs),
    lines: countLines(mcpMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/config.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/config.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Configuration loader and CLI parser for clibm-mcp.',
    language: 'rust',
    content: ensureUtf8Bom(mcpConfigRs),
    lines: countLines(mcpConfigRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/error.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/error.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Standard JSON-RPC 2.0 and MCP error definitions.',
    language: 'rust',
    content: ensureUtf8Bom(mcpErrorRs),
    lines: countLines(mcpErrorRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/protocol/mod.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/protocol/mod.rs',
    category: 'Plugins (TUI & Web)',
    description: 'MCP protocol module declaration.',
    language: 'rust',
    content: ensureUtf8Bom(mcpProtocolModRs),
    lines: countLines(mcpProtocolModRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/protocol/types.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/protocol/types.rs',
    category: 'Plugins (TUI & Web)',
    description: 'JSON-RPC request, response, and protocol version definitions.',
    language: 'rust',
    content: ensureUtf8Bom(mcpProtocolTypesRs),
    lines: countLines(mcpProtocolTypesRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/protocol/schema.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/protocol/schema.rs',
    category: 'Plugins (TUI & Web)',
    description: 'MCP schema data models for tools, resources, and prompts.',
    language: 'rust',
    content: ensureUtf8Bom(mcpProtocolSchemaRs),
    lines: countLines(mcpProtocolSchemaRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/backend/mod.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/backend/mod.rs',
    category: 'Plugins (TUI & Web)',
    description: 'CLIBM backend bridge module declaration.',
    language: 'rust',
    content: ensureUtf8Bom(mcpBackendModRs),
    lines: countLines(mcpBackendModRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/backend/bridge.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/backend/bridge.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Bridge to the local CLIBM CLI and CAS repository.',
    language: 'rust',
    content: ensureUtf8Bom(mcpBackendBridgeRs),
    lines: countLines(mcpBackendBridgeRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/mod.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/mod.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Shared MCP adapter orchestrating registries and request handling.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterModRs),
    lines: countLines(mcpAdapterModRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/discovery.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/discovery.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Dynamic plugin tool discovery from registry.json and live reloading monitor.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterDiscoveryRs),
    lines: countLines(mcpAdapterDiscoveryRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/dispatcher.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/dispatcher.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Request dispatcher routing JSON-RPC methods to registries.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterDispatcherRs),
    lines: countLines(mcpAdapterDispatcherRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/permissions.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/permissions.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Permission and sandbox policy manager for tools and resources.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterPermissionsRs),
    lines: countLines(mcpAdapterPermissionsRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/tools.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/tools.rs',
    category: 'Plugins (TUI & Web)',
    description: 'MCP tool registry with clibm_status, task_list, task_add, board_move, workspace_switch.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterToolsRs),
    lines: countLines(mcpAdapterToolsRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/validator.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/validator.rs',
    category: 'Plugins (TUI & Web)',
    description: 'JSON Schema validator for MCP dynamic plugin tools.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterValidatorRs),
    lines: countLines(mcpAdapterValidatorRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/resources.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/resources.rs',
    category: 'Plugins (TUI & Web)',
    description: 'MCP resource registry for staged trees, repo config, and Kanban boards.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterResourcesRs),
    lines: countLines(mcpAdapterResourcesRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/adapter/prompts.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/adapter/prompts.rs',
    category: 'Plugins (TUI & Web)',
    description: 'MCP prompt registry providing guided planning and commit summary templates.',
    language: 'rust',
    content: ensureUtf8Bom(mcpAdapterPromptsRs),
    lines: countLines(mcpAdapterPromptsRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/transport/mod.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/transport/mod.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Pluggable transport trait and declarations.',
    language: 'rust',
    content: ensureUtf8Bom(mcpTransportModRs),
    lines: countLines(mcpTransportModRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/transport/stdio.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/transport/stdio.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Zero-overhead STDIO JSON-RPC 2.0 transport for IDEs and CLI agents.',
    language: 'rust',
    content: ensureUtf8Bom(mcpTransportStdioRs),
    lines: countLines(mcpTransportStdioRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/src/transport/http.rs',
    zipPath: 'clibm/plugins/clibm-mcp/src/transport/http.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Local HTTP JSON-RPC 2.0 server transport using tiny_http.',
    language: 'rust',
    content: ensureUtf8Bom(mcpTransportHttpRs),
    lines: countLines(mcpTransportHttpRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/README.md',
    zipPath: 'clibm/plugins/clibm-mcp/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for configuring clibm-mcp transport options (both, stdio, http) and connecting AI agents.',
    language: 'markdown',
    content: ensureUtf8Bom(mcpReadmeMd),
    lines: countLines(mcpReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/install.sh',
    zipPath: 'clibm/plugins/clibm-mcp/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-mcp letting the user choose both (default), stdio, or http transport.',
    language: 'bash',
    content: ensureUtf8Bom(mcpInstallSh),
    lines: countLines(mcpInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/install.ps1',
    zipPath: 'clibm/plugins/clibm-mcp/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-mcp letting the user choose both (default), stdio, or http.',
    language: 'powershell',
    content: ensureUtf8Bom(mcpInstallPs1),
    lines: countLines(mcpInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-mcp/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-mcp with interactive keep/purge config choice.',
    language: 'bash',
    content: ensureUtf8Bom(mcpUninstallSh),
    lines: countLines(mcpUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-mcp/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-mcp/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-mcp.',
    language: 'powershell',
    content: ensureUtf8Bom(mcpUninstallPs1),
    lines: countLines(mcpUninstallPs1),
    isCoreOnly: false,
  },

  // Standalone AI DOCS Plugin (plugins/clibm-docs)
  {
    relativePath: 'plugins/clibm-docs/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-docs/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the AI Documentation & Context Teacher plugin (clibm-docs).',
    language: 'toml',
    content: ensureUtf8Bom(docsCargoToml),
    lines: countLines(docsCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-docs/src/main.rs',
    zipPath: 'clibm/plugins/clibm-docs/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Generates AGENTS.md, CLAUDE.md, .cursorrules, and stdout context manuals to teach AI agents to use CLIBM.',
    language: 'rust',
    content: ensureUtf8Bom(docsMainRs),
    lines: countLines(docsMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-docs/README.md',
    zipPath: 'clibm/plugins/clibm-docs/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for building and running clibm-docs in ongoing projects.',
    language: 'markdown',
    content: ensureUtf8Bom(docsReadmeMd),
    lines: countLines(docsReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-docs/install.sh',
    zipPath: 'clibm/plugins/clibm-docs/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-docs with AI doc format and auto-sync choices.',
    language: 'bash',
    content: ensureUtf8Bom(docsInstallSh),
    lines: countLines(docsInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-docs/install.ps1',
    zipPath: 'clibm/plugins/clibm-docs/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-docs.',
    language: 'powershell',
    content: ensureUtf8Bom(docsInstallPs1),
    lines: countLines(docsInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-docs/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-docs/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-docs.',
    language: 'bash',
    content: ensureUtf8Bom(docsUninstallSh),
    lines: countLines(docsUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-docs/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-docs/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-docs.',
    language: 'powershell',
    content: ensureUtf8Bom(docsUninstallPs1),
    lines: countLines(docsUninstallPs1),
    isCoreOnly: false,
  },

  // Extracted Default Plugin 1: plugins/clibm-dep-audit
  {
    relativePath: 'plugins/clibm-dep-audit/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-dep-audit/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the extracted DAG & Critical Path Auditor plugin.',
    language: 'toml',
    content: ensureUtf8Bom(depAuditCargoToml),
    lines: countLines(depAuditCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-dep-audit/src/main.rs',
    zipPath: 'clibm/plugins/clibm-dep-audit/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone DFS circular dependency and critical-path blocker analyzer crate.',
    language: 'rust',
    content: ensureUtf8Bom(depAuditMainRs),
    lines: countLines(depAuditMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-dep-audit/README.md',
    zipPath: 'clibm/plugins/clibm-dep-audit/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for installing and running clibm-dep-audit from plugins/clibm-dep-audit.',
    language: 'markdown',
    content: ensureUtf8Bom(depAuditReadmeMd),
    lines: countLines(depAuditReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-dep-audit/install.sh',
    zipPath: 'clibm/plugins/clibm-dep-audit/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-dep-audit.',
    language: 'bash',
    content: ensureUtf8Bom(depAuditInstallSh),
    lines: countLines(depAuditInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-dep-audit/install.ps1',
    zipPath: 'clibm/plugins/clibm-dep-audit/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-dep-audit.',
    language: 'powershell',
    content: ensureUtf8Bom(depAuditInstallPs1),
    lines: countLines(depAuditInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-dep-audit/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-dep-audit/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-dep-audit.',
    language: 'bash',
    content: ensureUtf8Bom(depAuditUninstallSh),
    lines: countLines(depAuditUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-dep-audit/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-dep-audit/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-dep-audit.',
    language: 'powershell',
    content: ensureUtf8Bom(depAuditUninstallPs1),
    lines: countLines(depAuditUninstallPs1),
    isCoreOnly: false,
  },

  // Extracted Default Plugin 2: plugins/clibm-export-md
  {
    relativePath: 'plugins/clibm-export-md/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-export-md/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the extracted Markdown Sprint Exporter plugin.',
    language: 'toml',
    content: ensureUtf8Bom(exportMdCargoToml),
    lines: countLines(exportMdCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-export-md/src/main.rs',
    zipPath: 'clibm/plugins/clibm-export-md/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Markdown table and sprint report generator over BLAKE3 CAS snapshots.',
    language: 'rust',
    content: ensureUtf8Bom(exportMdMainRs),
    lines: countLines(exportMdMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-export-md/README.md',
    zipPath: 'clibm/plugins/clibm-export-md/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for installing and running clibm-export-md from plugins/clibm-export-md.',
    language: 'markdown',
    content: ensureUtf8Bom(exportMdReadmeMd),
    lines: countLines(exportMdReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-export-md/install.sh',
    zipPath: 'clibm/plugins/clibm-export-md/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-export-md.',
    language: 'bash',
    content: ensureUtf8Bom(exportMdInstallSh),
    lines: countLines(exportMdInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-export-md/install.ps1',
    zipPath: 'clibm/plugins/clibm-export-md/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-export-md.',
    language: 'powershell',
    content: ensureUtf8Bom(exportMdInstallPs1),
    lines: countLines(exportMdInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-export-md/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-export-md/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-export-md.',
    language: 'bash',
    content: ensureUtf8Bom(exportMdUninstallSh),
    lines: countLines(exportMdUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-export-md/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-export-md/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-export-md.',
    language: 'powershell',
    content: ensureUtf8Bom(exportMdUninstallPs1),
    lines: countLines(exportMdUninstallPs1),
    isCoreOnly: false,
  },

  // Extracted Default Plugin 3: plugins/clibm-workflow-lint
  {
    relativePath: 'plugins/clibm-workflow-lint/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-workflow-lint/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the extracted Kanban WIP & Policy Linter plugin.',
    language: 'toml',
    content: ensureUtf8Bom(workflowLintCargoToml),
    lines: countLines(workflowLintCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-workflow-lint/src/main.rs',
    zipPath: 'clibm/plugins/clibm-workflow-lint/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Kanban WIP limit, assignee policy, and dependency completion validator crate.',
    language: 'rust',
    content: ensureUtf8Bom(workflowLintMainRs),
    lines: countLines(workflowLintMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-workflow-lint/README.md',
    zipPath: 'clibm/plugins/clibm-workflow-lint/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for installing and running clibm-workflow-lint from plugins/clibm-workflow-lint.',
    language: 'markdown',
    content: ensureUtf8Bom(workflowLintReadmeMd),
    lines: countLines(workflowLintReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-workflow-lint/install.sh',
    zipPath: 'clibm/plugins/clibm-workflow-lint/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-workflow-lint.',
    language: 'bash',
    content: ensureUtf8Bom(workflowLintInstallSh),
    lines: countLines(workflowLintInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-workflow-lint/install.ps1',
    zipPath: 'clibm/plugins/clibm-workflow-lint/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-workflow-lint.',
    language: 'powershell',
    content: ensureUtf8Bom(workflowLintInstallPs1),
    lines: countLines(workflowLintInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-workflow-lint/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-workflow-lint/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-workflow-lint.',
    language: 'bash',
    content: ensureUtf8Bom(workflowLintUninstallSh),
    lines: countLines(workflowLintUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-workflow-lint/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-workflow-lint/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-workflow-lint.',
    language: 'powershell',
    content: ensureUtf8Bom(workflowLintUninstallPs1),
    lines: countLines(workflowLintUninstallPs1),
    isCoreOnly: false,
  },

  // TUI & Web Plugins
  {
    relativePath: 'plugins/clibm-tui/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-tui/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the Ratatui + Crossterm terminal UI plugin (built from its own folder).',
    language: 'toml',
    content: ensureUtf8Bom(tuiCargoToml),
    lines: countLines(tuiCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-tui/README.md',
    zipPath: 'clibm/plugins/clibm-tui/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for building clibm-tui in its own folder and running it inside ongoing projects.',
    language: 'markdown',
    content: ensureUtf8Bom(tuiReadmeMd),
    lines: countLines(tuiReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-tui/src/main.rs',
    zipPath: 'clibm/plugins/clibm-tui/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Self-contained Ratatui split-pane TUI plugin attaching to ongoing .clibm projects.',
    language: 'rust',
    content: ensureUtf8Bom(tuiMainRs),
    lines: countLines(tuiMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-tui/install.sh',
    zipPath: 'clibm/plugins/clibm-tui/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-tui.',
    language: 'bash',
    content: ensureUtf8Bom(tuiInstallSh),
    lines: countLines(tuiInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-tui/install.ps1',
    zipPath: 'clibm/plugins/clibm-tui/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-tui.',
    language: 'powershell',
    content: ensureUtf8Bom(tuiInstallPs1),
    lines: countLines(tuiInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-tui/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-tui/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-tui.',
    language: 'bash',
    content: ensureUtf8Bom(tuiUninstallSh),
    lines: countLines(tuiUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-tui/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-tui/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-tui.',
    language: 'powershell',
    content: ensureUtf8Bom(tuiUninstallPs1),
    lines: countLines(tuiUninstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/Cargo.toml',
    zipPath: 'clibm/plugins/clibm-web/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Standalone Cargo manifest for the single-binary local HTTP server and embedded web UI plugin.',
    language: 'toml',
    content: ensureUtf8Bom(webCargoToml),
    lines: countLines(webCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/README.md',
    zipPath: 'clibm/plugins/clibm-web/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Documentation for building clibm-web in its own folder and running it inside ongoing projects.',
    language: 'markdown',
    content: ensureUtf8Bom(webReadmeMd),
    lines: countLines(webReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/src/main.rs',
    zipPath: 'clibm/plugins/clibm-web/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Self-contained Rust HTTP server binding to 127.0.0.1:7878 over any ongoing .clibm project.',
    language: 'rust',
    content: ensureUtf8Bom(webMainRs),
    lines: countLines(webMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/src/ui.html',
    zipPath: 'clibm/plugins/clibm-web/src/ui.html',
    category: 'Plugins (TUI & Web)',
    description: 'Zero-dependency self-hosted HTML/JS application embedded into the clibm-web binary via include_str!.',
    language: 'html',
    content: ensureUtf8Bom(webUiHtml),
    lines: countLines(webUiHtml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/install.sh',
    zipPath: 'clibm/plugins/clibm-web/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer for clibm-web.',
    language: 'bash',
    content: ensureUtf8Bom(webInstallSh),
    lines: countLines(webInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/install.ps1',
    zipPath: 'clibm/plugins/clibm-web/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer for clibm-web.',
    language: 'powershell',
    content: ensureUtf8Bom(webInstallPs1),
    lines: countLines(webInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/uninstall.sh',
    zipPath: 'clibm/plugins/clibm-web/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller for clibm-web.',
    language: 'bash',
    content: ensureUtf8Bom(webUninstallSh),
    lines: countLines(webUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/clibm-web/uninstall.ps1',
    zipPath: 'clibm/plugins/clibm-web/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller for clibm-web.',
    language: 'powershell',
    content: ensureUtf8Bom(webUninstallPs1),
    lines: countLines(webUninstallPs1),
    isCoreOnly: false,
  },

  // Plugin Template
  {
    relativePath: 'plugins/plugin-template/Cargo.toml',
    zipPath: 'clibm/plugins/plugin-template/Cargo.toml',
    category: 'Plugins (TUI & Web)',
    description: 'Official starter Cargo manifest for creating new standalone CLIBM plugins.',
    language: 'toml',
    content: ensureUtf8Bom(tplCargoToml),
    lines: countLines(tplCargoToml),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/plugin-template/src/main.rs',
    zipPath: 'clibm/plugins/plugin-template/src/main.rs',
    category: 'Plugins (TUI & Web)',
    description: 'Official starter Rust source implementing CLIBM_PLUGIN_CONTRACT_V1 JSON stdin/stdout and direct project inspection.',
    language: 'rust',
    content: ensureUtf8Bom(tplMainRs),
    lines: countLines(tplMainRs),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/plugin-template/README.md',
    zipPath: 'clibm/plugins/plugin-template/README.md',
    category: 'Plugins (TUI & Web)',
    description: 'Developer guide for scaffolding, compiling, installing, and running custom CLIBM plugins.',
    language: 'markdown',
    content: ensureUtf8Bom(tplReadmeMd),
    lines: countLines(tplReadmeMd),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/plugin-template/install.sh',
    zipPath: 'clibm/plugins/plugin-template/install.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Linux installer script for the plugin template.',
    language: 'bash',
    content: ensureUtf8Bom(tplInstallSh),
    lines: countLines(tplInstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/plugin-template/install.ps1',
    zipPath: 'clibm/plugins/plugin-template/install.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM interactive Windows PowerShell installer script for the plugin template.',
    language: 'powershell',
    content: ensureUtf8Bom(tplInstallPs1),
    lines: countLines(tplInstallPs1),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/plugin-template/uninstall.sh',
    zipPath: 'clibm/plugins/plugin-template/uninstall.sh',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Linux uninstaller script for removing the compiled template plugin binary.',
    language: 'bash',
    content: ensureUtf8Bom(tplUninstallSh),
    lines: countLines(tplUninstallSh),
    isCoreOnly: false,
  },
  {
    relativePath: 'plugins/plugin-template/uninstall.ps1',
    zipPath: 'clibm/plugins/plugin-template/uninstall.ps1',
    category: 'Plugins (TUI & Web)',
    description: 'UTF-8-BOM Windows PowerShell uninstaller script for removing the compiled template plugin binary.',
    language: 'powershell',
    content: ensureUtf8Bom(tplUninstallPs1),
    lines: countLines(tplUninstallPs1),
    isCoreOnly: false,
  },
];

export const OFFICIAL_PLUGIN_PACKAGES: DownloadablePluginPackage[] = [
  {
    id: 'clibm-mcp',
    name: 'CLIBM Self-Hosted MCP Server for AI',
    crateName: 'clibm-mcp',
    version: '0.2.0',
    category: 'AI & MCP',
    summary:
      'Self-hosted Model Context Protocol (MCP) server for AI agents with modular STDIO and HTTP transports and shared adapter architecture.',
    architectureNotes:
      'Modular crate in plugins/clibm-mcp. Features compile-time transport gating (transport-stdio, transport-http) with a decoupled Shared Adapter (ToolRegistry, ResourceRegistry, PromptRegistry, PermissionManager, RequestDispatcher). All install/uninstall scripts are UTF-8-BOM encoded.',
    stack: 'Rust 2021 · MCP 2024-11-05 · Modular Transports (stdio / tiny_http) · UTF-8-BOM',
    hooks: ['mcp-stdio', 'mcp-http-sse', 'on-ai-tool-call'],
    installCommand: 'cd plugins/clibm-mcp && ./install.sh --transport both',
    cliCommand: 'clibm plugin run mcp',
    binaryInvoke: 'clibm-mcp',
    zipFilename: 'clibm-mcp-v0.2.0.zip',
    installPrompt: {
      flagName: '--transport',
      ps1ParamName: '-Transport',
      promptTitle: 'Select MCP Transport Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-mcp, the installer prompts you to choose whether to enable both transports (default) or only one:',
      defaultValue: 'both',
      options: [
        {
          value: 'both',
          label: '1) Both Transports (Default)',
          detail: 'Enable stdio JSON-RPC 2.0 + Local HTTP SSE Server (127.0.0.1:7879)',
        },
        {
          value: 'stdio',
          label: '2) stdio JSON-RPC 2.0 Only',
          detail: 'Enable only stdio transport (zero network/HTTP dependencies)',
        },
        {
          value: 'http',
          label: '3) Local HTTP Server Only',
          detail: 'Enable only Local HTTP JSON-RPC 2.0 Server on 127.0.0.1:7879',
        },
      ],
    },
    files: [
      { path: 'clibm-mcp/Cargo.toml', content: ensureUtf8Bom(mcpCargoToml) },
      { path: 'clibm-mcp/README.md', content: ensureUtf8Bom(mcpReadmeMd) },
      { path: 'clibm-mcp/install.sh', content: ensureUtf8Bom(mcpInstallSh) },
      { path: 'clibm-mcp/install.ps1', content: ensureUtf8Bom(mcpInstallPs1) },
      { path: 'clibm-mcp/uninstall.sh', content: ensureUtf8Bom(mcpUninstallSh) },
      { path: 'clibm-mcp/uninstall.ps1', content: ensureUtf8Bom(mcpUninstallPs1) },
      { path: 'clibm-mcp/src/main.rs', content: ensureUtf8Bom(mcpMainRs) },
      { path: 'clibm-mcp/src/config.rs', content: ensureUtf8Bom(mcpConfigRs) },
      { path: 'clibm-mcp/src/error.rs', content: ensureUtf8Bom(mcpErrorRs) },
      { path: 'clibm-mcp/src/protocol/mod.rs', content: ensureUtf8Bom(mcpProtocolModRs) },
      { path: 'clibm-mcp/src/protocol/types.rs', content: ensureUtf8Bom(mcpProtocolTypesRs) },
      { path: 'clibm-mcp/src/protocol/schema.rs', content: ensureUtf8Bom(mcpProtocolSchemaRs) },
      { path: 'clibm-mcp/src/backend/mod.rs', content: ensureUtf8Bom(mcpBackendModRs) },
      { path: 'clibm-mcp/src/backend/bridge.rs', content: ensureUtf8Bom(mcpBackendBridgeRs) },
      { path: 'clibm-mcp/src/adapter/mod.rs', content: ensureUtf8Bom(mcpAdapterModRs) },
      { path: 'clibm-mcp/src/adapter/discovery.rs', content: ensureUtf8Bom(mcpAdapterDiscoveryRs) },
      { path: 'clibm-mcp/src/adapter/dispatcher.rs', content: ensureUtf8Bom(mcpAdapterDispatcherRs) },
      { path: 'clibm-mcp/src/adapter/permissions.rs', content: ensureUtf8Bom(mcpAdapterPermissionsRs) },
      { path: 'clibm-mcp/src/adapter/tools.rs', content: ensureUtf8Bom(mcpAdapterToolsRs) },
      { path: 'clibm-mcp/src/adapter/validator.rs', content: ensureUtf8Bom(mcpAdapterValidatorRs) },
      { path: 'clibm-mcp/src/adapter/resources.rs', content: ensureUtf8Bom(mcpAdapterResourcesRs) },
      { path: 'clibm-mcp/src/adapter/prompts.rs', content: ensureUtf8Bom(mcpAdapterPromptsRs) },
      { path: 'clibm-mcp/src/transport/mod.rs', content: ensureUtf8Bom(mcpTransportModRs) },
      { path: 'clibm-mcp/src/transport/stdio.rs', content: ensureUtf8Bom(mcpTransportStdioRs) },
      { path: 'clibm-mcp/src/transport/http.rs', content: ensureUtf8Bom(mcpTransportHttpRs) },
    ],
  },
  {
    id: 'clibm-docs',
    name: 'CLIBM AI Context & DOCS Teacher',
    crateName: 'clibm-docs',
    version: '0.1.0',
    category: 'AI & MCP',
    summary:
      'Teaches AI agents (Claude Code, Cursor, Windsurf, Copilot) how to operate CLIBM by outputting structured AI context to stdout and generating AGENTS.md, CLAUDE.md, .cursorrules, and .mcp.json.',
    architectureNotes:
      'Standalone crate in plugins/clibm-docs. Inspects the live .clibm/ repository state, active workspace, and installed plugins to synthesize accurate AI instructions. All install/uninstall scripts are UTF-8-BOM encoded.',
    stack: 'Rust 2021 · AGENTS.md / CLAUDE.md / .cursorrules Generator · UTF-8-BOM',
    hooks: ['ai-docs-generator', 'on-init'],
    installCommand: 'cd plugins/clibm-docs && ./install.sh --format all',
    cliCommand: 'clibm plugin run docs -- --all',
    binaryInvoke: 'clibm-docs --all',
    zipFilename: 'clibm-docs-v0.1.0.zip',
    installPrompt: {
      flagName: '--format',
      ps1ParamName: '-Format',
      promptTitle: 'Select AI Context Document Format (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-docs, the installer prompts you to select which AI instruction files to generate:',
      defaultValue: 'all',
      options: [
        {
          value: 'all',
          label: '1) All AI Context Files (Default)',
          detail: 'Generate AGENTS.md, CLAUDE.md, .cursorrules, and .mcp.json',
        },
        {
          value: 'agents',
          label: '2) AGENTS.md Only',
          detail: 'Universal AI agent instruction manual for the repository',
        },
        {
          value: 'claude',
          label: '3) CLAUDE.md Only',
          detail: 'Claude Code & Anthropic agent workflow guide',
        },
        {
          value: 'cursorrules',
          label: '4) .cursorrules Only',
          detail: 'Cursor & Windsurf IDE rules file',
        },
      ],
    },
    files: [
      { path: 'clibm-docs/Cargo.toml', content: ensureUtf8Bom(docsCargoToml) },
      { path: 'clibm-docs/README.md', content: ensureUtf8Bom(docsReadmeMd) },
      { path: 'clibm-docs/install.sh', content: ensureUtf8Bom(docsInstallSh) },
      { path: 'clibm-docs/install.ps1', content: ensureUtf8Bom(docsInstallPs1) },
      { path: 'clibm-docs/uninstall.sh', content: ensureUtf8Bom(docsUninstallSh) },
      { path: 'clibm-docs/uninstall.ps1', content: ensureUtf8Bom(docsUninstallPs1) },
      { path: 'clibm-docs/src/main.rs', content: ensureUtf8Bom(docsMainRs) },
    ],
  },
  {
    id: 'clibm-dep-audit',
    name: 'DAG Critical Path & Cycle Auditor',
    crateName: 'clibm-dep-audit',
    version: '0.1.0',
    category: 'Analysis & Audit',
    summary:
      'Extracted from core into plugins/clibm-dep-audit: audits the task dependency graph in .clibm/ to identify critical-path blockers and prevent circular dependency chains.',
    architectureNotes:
      'Standalone Cargo crate in plugins/clibm-dep-audit. Install from its folder via ./install.sh (or select Core + Default Plugins during core installation), then run via `clibm plugin run dep-audit`.',
    stack: 'Rust 2021 · DFS Cycle Detector · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8-BOM',
    hooks: ['pre-commit', 'on-task-depend'],
    installCommand: 'cd plugins/clibm-dep-audit && ./install.sh',
    cliCommand: 'clibm plugin run dep-audit',
    binaryInvoke: 'clibm-dep-audit',
    zipFilename: 'clibm-dep-audit-v0.1.0.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select DAG Audit Hook Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-dep-audit, choose whether to run automatically on commits/dependencies or manually:',
      defaultValue: 'pre-commit,on-task-depend',
      options: [
        {
          value: 'pre-commit,on-task-depend',
          label: '1) Automatic Hooks (Default)',
          detail: 'Audit DAG cycles automatically on pre-commit and on-task-depend',
        },
        {
          value: 'manual-run',
          label: '2) Manual Audit Only',
          detail: 'Run only when invoked via clibm plugin run dep-audit',
        },
      ],
    },
    files: [
      { path: 'clibm-dep-audit/Cargo.toml', content: ensureUtf8Bom(depAuditCargoToml) },
      { path: 'clibm-dep-audit/README.md', content: ensureUtf8Bom(depAuditReadmeMd) },
      { path: 'clibm-dep-audit/install.sh', content: ensureUtf8Bom(depAuditInstallSh) },
      { path: 'clibm-dep-audit/install.ps1', content: ensureUtf8Bom(depAuditInstallPs1) },
      { path: 'clibm-dep-audit/uninstall.sh', content: ensureUtf8Bom(depAuditUninstallSh) },
      { path: 'clibm-dep-audit/uninstall.ps1', content: ensureUtf8Bom(depAuditUninstallPs1) },
      { path: 'clibm-dep-audit/src/main.rs', content: ensureUtf8Bom(depAuditMainRs) },
    ],
  },
  {
    id: 'clibm-export-md',
    name: 'Markdown & Portable Sprint Exporter',
    crateName: 'clibm-export-md',
    version: '0.1.0',
    category: 'Export & Interop',
    summary:
      'Extracted from core into plugins/clibm-export-md: transforms any immutable BLAKE3 tree snapshot in an ongoing project into clean Markdown tables and sprint reports.',
    architectureNotes:
      'Standalone Cargo crate in plugins/clibm-export-md. Reads canonical task and board payloads from .clibm/objects without mutating state.',
    stack: 'Rust 2021 · Canonical JSON Reader · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8-BOM',
    hooks: ['on-export', 'post-commit'],
    installCommand: 'cd plugins/clibm-export-md && ./install.sh',
    cliCommand: 'clibm plugin run export-md',
    binaryInvoke: 'clibm-export-md',
    zipFilename: 'clibm-export-md-v0.1.0.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select Markdown Export Hook Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-export-md, choose whether to trigger Markdown export hooks automatically or manually:',
      defaultValue: 'on-export,post-commit',
      options: [
        {
          value: 'on-export,post-commit',
          label: '1) Automatic Export Hooks (Default)',
          detail: 'Run on-export and post-commit to keep sprint Markdown reports updated',
        },
        {
          value: 'manual-run',
          label: '2) Manual Export Only',
          detail: 'Run only when invoked via clibm plugin run export-md',
        },
      ],
    },
    files: [
      { path: 'clibm-export-md/Cargo.toml', content: ensureUtf8Bom(exportMdCargoToml) },
      { path: 'clibm-export-md/README.md', content: ensureUtf8Bom(exportMdReadmeMd) },
      { path: 'clibm-export-md/install.sh', content: ensureUtf8Bom(exportMdInstallSh) },
      { path: 'clibm-export-md/install.ps1', content: ensureUtf8Bom(exportMdInstallPs1) },
      { path: 'clibm-export-md/uninstall.sh', content: ensureUtf8Bom(exportMdUninstallSh) },
      { path: 'clibm-export-md/uninstall.ps1', content: ensureUtf8Bom(exportMdUninstallPs1) },
      { path: 'clibm-export-md/src/main.rs', content: ensureUtf8Bom(exportMdMainRs) },
    ],
  },
  {
    id: 'clibm-workflow-lint',
    name: 'Kanban WIP & Policy Linter',
    crateName: 'clibm-workflow-lint',
    version: '0.1.0',
    category: 'Workflow Policy',
    summary:
      'Extracted from core into plugins/clibm-workflow-lint: enforces Kanban column Work-In-Progress (WIP) limits, assignee requirements, and unblocked dependency policies.',
    architectureNotes:
      'Standalone Cargo crate in plugins/clibm-workflow-lint. Hooks into pre-move and pre-commit events to validate workflow invariants defined in KanbanBoardPayload.',
    stack: 'Rust 2021 · Workflow Policy Engine · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8-BOM',
    hooks: ['pre-move', 'pre-commit'],
    installCommand: 'cd plugins/clibm-workflow-lint && ./install.sh',
    cliCommand: 'clibm plugin run workflow-lint',
    binaryInvoke: 'clibm-workflow-lint',
    zipFilename: 'clibm-workflow-lint-v0.1.0.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select Workflow Policy Linter Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-workflow-lint, select whether to enforce WIP policies automatically or on manual runs:',
      defaultValue: 'pre-move,pre-commit',
      options: [
        {
          value: 'pre-move,pre-commit',
          label: '1) Automatic Policy Enforcement (Default)',
          detail: 'Validate WIP limits and assignees on pre-move and pre-commit',
        },
        {
          value: 'manual-run',
          label: '2) Manual Audit Only',
          detail: 'Run only when invoked via clibm plugin run workflow-lint',
        },
      ],
    },
    files: [
      { path: 'clibm-workflow-lint/Cargo.toml', content: ensureUtf8Bom(workflowLintCargoToml) },
      { path: 'clibm-workflow-lint/README.md', content: ensureUtf8Bom(workflowLintReadmeMd) },
      { path: 'clibm-workflow-lint/install.sh', content: ensureUtf8Bom(workflowLintInstallSh) },
      { path: 'clibm-workflow-lint/install.ps1', content: ensureUtf8Bom(workflowLintInstallPs1) },
      { path: 'clibm-workflow-lint/uninstall.sh', content: ensureUtf8Bom(workflowLintUninstallSh) },
      { path: 'clibm-workflow-lint/uninstall.ps1', content: ensureUtf8Bom(workflowLintUninstallPs1) },
      { path: 'clibm-workflow-lint/src/main.rs', content: ensureUtf8Bom(workflowLintMainRs) },
    ],
  },
  {
    id: 'clibm-tui',
    name: 'CLIBM Ratatui TUI',
    crateName: 'clibm-tui',
    version: '0.1.0',
    category: 'Interface Plugin',
    summary:
      'Standalone Ratatui + Crossterm Terminal User Interface crate built from its own folder and executed inside any ongoing .clibm project.',
    architectureNotes:
      'Self-contained Cargo crate with zero relative path coupling. Install from inside plugins/clibm-tui via ./install.sh (or clibm plugin install .), then run inside any ongoing project via `clibm plugin run tui` or `clibm-tui`.',
    stack: 'Rust 2021 · ratatui 0.29 · crossterm 0.28 · blake3 1.5 · UTF-8-BOM',
    hooks: ['interactive-tui', 'on-board-move', 'pre-commit'],
    installCommand: 'cd plugins/clibm-tui && ./install.sh',
    cliCommand: 'clibm plugin run tui',
    binaryInvoke: 'clibm-tui',
    zipFilename: 'clibm-tui-v0.1.0.zip',
    installPrompt: {
      flagName: '--keymap',
      ps1ParamName: '-Keymap',
      promptTitle: 'Select TUI Keybinding Preset (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-tui, choose your preferred terminal keybinding preset:',
      defaultValue: 'vim-arrows',
      options: [
        {
          value: 'vim-arrows',
          label: '1) Vim (h/j/k/l) + Arrow Keys (Default)',
          detail: 'Full Vim modal navigation alongside standard arrow keys',
        },
        {
          value: 'arrows',
          label: '2) Standard Arrow Keys Only',
          detail: 'Arrow keys, Enter, Tab, and function keys',
        },
      ],
    },
    files: [
      { path: 'clibm-tui/Cargo.toml', content: ensureUtf8Bom(tuiCargoToml) },
      { path: 'clibm-tui/README.md', content: ensureUtf8Bom(tuiReadmeMd) },
      { path: 'clibm-tui/install.sh', content: ensureUtf8Bom(tuiInstallSh) },
      { path: 'clibm-tui/install.ps1', content: ensureUtf8Bom(tuiInstallPs1) },
      { path: 'clibm-tui/uninstall.sh', content: ensureUtf8Bom(tuiUninstallSh) },
      { path: 'clibm-tui/uninstall.ps1', content: ensureUtf8Bom(tuiUninstallPs1) },
      { path: 'clibm-tui/src/main.rs', content: ensureUtf8Bom(tuiMainRs) },
    ],
  },
  {
    id: 'clibm-web',
    name: 'CLIBM Self-Hosted Web Server',
    crateName: 'clibm-web',
    version: '0.1.0',
    category: 'Interface Plugin',
    summary:
      'Standalone single-binary local HTTP server (127.0.0.1:7878) built from its own folder and serving an embedded web workbench inside any ongoing .clibm project.',
    architectureNotes:
      'Embeds ui.html at compile time via include_str! and discovers the ongoing .clibm repository in the current or parent directory. Install from plugins/clibm-web via ./install.sh, then run inside your project via `clibm plugin run web` or `clibm-web --port 7878`.',
    stack: 'Rust 2021 · tiny_http 0.12 · blake3 1.5 · Embedded HTML5 · UTF-8-BOM',
    hooks: ['http-bridge', 'on-state-query', 'on-task-mutate'],
    installCommand: 'cd plugins/clibm-web && ./install.sh',
    cliCommand: 'clibm plugin run web -- --port 7878',
    binaryInvoke: 'clibm-web --port 7878',
    zipFilename: 'clibm-web-v0.1.0.zip',
    installPrompt: {
      flagName: '--port',
      ps1ParamName: '-Port',
      promptTitle: 'Select Default Local HTTP Server Port (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-web, choose the default local port for the self-hosted web server:',
      defaultValue: '7878',
      options: [
        {
          value: '7878',
          label: '1) Port 7878 (Default)',
          detail: 'Bind self-hosted web workbench to http://127.0.0.1:7878',
        },
        {
          value: '8080',
          label: '2) Port 8080',
          detail: 'Bind self-hosted web workbench to http://127.0.0.1:8080',
        },
      ],
    },
    files: [
      { path: 'clibm-web/Cargo.toml', content: ensureUtf8Bom(webCargoToml) },
      { path: 'clibm-web/README.md', content: ensureUtf8Bom(webReadmeMd) },
      { path: 'clibm-web/install.sh', content: ensureUtf8Bom(webInstallSh) },
      { path: 'clibm-web/install.ps1', content: ensureUtf8Bom(webInstallPs1) },
      { path: 'clibm-web/uninstall.sh', content: ensureUtf8Bom(webUninstallSh) },
      { path: 'clibm-web/uninstall.ps1', content: ensureUtf8Bom(webUninstallPs1) },
      { path: 'clibm-web/src/main.rs', content: ensureUtf8Bom(webMainRs) },
      { path: 'clibm-web/src/ui.html', content: ensureUtf8Bom(webUiHtml) },
    ],
  },
  {
    id: 'plugin-template',
    name: 'Official Rust Plugin Starter Template',
    crateName: 'clibm-plugin-template',
    version: '0.1.0',
    category: 'Plugin Template',
    summary:
      'Ready-to-compile standalone Rust plugin template implementing CLIBM_PLUGIN_CONTRACT_V1 stdin/stdout hooks, .clibm/ project inspection, and UTF-8-BOM folder-scoped install/uninstall scripts.',
    architectureNotes:
      'Use `clibm plugin new <name>` or customize this template directly to create new CLIBM extensions in under 60 seconds. Includes UTF-8-BOM encoded Cargo.toml, src/main.rs, README.md, install.sh, install.ps1, uninstall.sh, and uninstall.ps1.',
    stack: 'Rust 2021 · serde_json · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8-BOM',
    hooks: ['pre-commit', 'post-commit', 'on-task-move', 'on-workspace-switch'],
    installCommand: 'cd plugins/plugin-template && ./install.sh',
    cliCommand: 'clibm plugin new my-plugin',
    binaryInvoke: 'clibm plugin run clibm-plugin-template',
    zipFilename: 'clibm-plugin-template-v0.1.0.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select Template Hook Subscription Profile (install.sh / install.ps1)',
      promptDescription:
        'When installing a plugin created from plugin-template, the installer prompts you for the hook subscription profile:',
      defaultValue: 'default-hooks',
      options: [
        {
          value: 'default-hooks',
          label: '1) Default Contract Hooks (Default)',
          detail: 'Subscribe to pre-commit & on-task-move lifecycle events',
        },
        {
          value: 'manual-only',
          label: '2) Manual Invocation Only',
          detail: 'Run only when explicitly invoked via clibm plugin run <name>',
        },
        {
          value: 'all-hooks',
          label: '3) All Lifecycle Hooks',
          detail: 'Subscribe to pre-commit, post-commit, on-task-move, on-workspace-switch, on-export',
        },
      ],
    },
    files: [
      { path: 'plugin-template/Cargo.toml', content: ensureUtf8Bom(tplCargoToml) },
      { path: 'plugin-template/README.md', content: ensureUtf8Bom(tplReadmeMd) },
      { path: 'plugin-template/src/main.rs', content: ensureUtf8Bom(tplMainRs) },
      { path: 'plugin-template/install.sh', content: ensureUtf8Bom(tplInstallSh) },
      { path: 'plugin-template/install.ps1', content: ensureUtf8Bom(tplInstallPs1) },
      { path: 'plugin-template/uninstall.sh', content: ensureUtf8Bom(tplUninstallSh) },
      { path: 'plugin-template/uninstall.ps1', content: ensureUtf8Bom(tplUninstallPs1) },
    ],
  },
];

/**
 * Dynamically generates the UTF-8-BOM encoded source and installation/uninstallation
 * files for a custom plugin based on the official template.
 */
export function buildCustomPluginTemplateFiles(
  rawName: string,
  description: string,
  hooks: string[]
): { crateName: string; files: ZipFileEntry[] } {
  const slug =
    rawName
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9-]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'custom-plugin';
  const crateName = slug.startsWith('clibm-') ? slug : `clibm-${slug}`;
  const cleanDesc =
    description.trim() || `Custom standalone CLIBM plugin (${crateName}) built from official template`;
  const cleanHooks = hooks.length > 0 ? hooks : ['pre-commit', 'on-task-move'];

  const customCargoToml = ensureUtf8Bom(
    tplCargoToml
      .replace(/clibm-plugin-template/g, crateName)
      .replace(
        'Official starter template for building standalone CLIBM plugins in Rust',
        cleanDesc.replace(/"/g, "'")
      )
  );

  const customMainRs = ensureUtf8Bom(
    tplMainRs
      .replace(/clibm-plugin-template/g, crateName)
      .replace(
        '//! CLIBM Official Plugin Starter Template',
        `//! Standalone CLIBM Plugin: ${crateName}\n//! Description: ${cleanDesc}\n//! Subscribed Hooks: ${cleanHooks.join(', ')}`
      )
  );

  const customReadme = ensureUtf8Bom(
    `# ${crateName} (v0.1.0)\n\n${cleanDesc}\n\n- **Contract:** \`CLIBM_PLUGIN_CONTRACT_V1\`\n- **Hooks:** \`${cleanHooks.join('`, `')}\`\n- **Encoding:** \`UTF-8 with BOM (EF BB BF)\`\n\n## Quick Install\n\n### Linux, macOS, WSL2\n\`\`\`bash\ncurl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/install.sh | bash\n\`\`\`\n\n### Windows (native, PowerShell)\n\`\`\`powershell\niex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/install.ps1)\n\`\`\`\n\n## Quick Uninstall\n\n### Linux, macOS, WSL2\n\`\`\`bash\ncurl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/uninstall.sh | bash\n\`\`\`\n\n### Windows (native, PowerShell)\n\`\`\`powershell\niex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/uninstall.ps1)\n\`\`\`\n\n## 1. Local Build & Install (From This Plugin Folder)\n\n\`\`\`bash\ncd ${crateName}\n./install.sh\n# Or on Windows PowerShell:\n.\\install.ps1\n\`\`\`\n\n## 2. Run Inside Any Ongoing CLIBM Project\n\n\`\`\`bash\nclibm plugin run ${crateName}\n# Or invoke the binary directly:\n${crateName}\n\`\`\`\n\n## 3. Local Uninstall\n\n\`\`\`bash\n./uninstall.sh\n# Or on Windows PowerShell:\n.\\uninstall.ps1\n\`\`\`\n`
  );

  const safeDesc = cleanDesc.replace(/"/g, "'");
  const hooksCsv = cleanHooks.join(',');
  const hooksPs1 = cleanHooks.map((h) => `"${h}"`).join(', ');

  const customInstallSh = ensureUtf8Bom(
    tplInstallSh
      .replace(/clibm-plugin-template/g, crateName)
      .replace('TPL_HOOKS="pre-commit,on-task-move"', `TPL_HOOKS="${hooksCsv}"`)
      .replace('PLUGIN_DESC="Standalone CLIBM plugin (${BIN_NAME})"', `PLUGIN_DESC="${safeDesc}"`)
  );
  const customInstallPs1 = ensureUtf8Bom(
    tplInstallPs1
      .replace(/clibm-plugin-template/g, crateName)
      .replace('"default-hooks" { @("pre-commit", "on-task-move") }', `"default-hooks" { @(${hooksPs1}) }`)
      .replace('$PluginDesc = "Standalone CLIBM plugin ($BinName)"', `$PluginDesc = "${safeDesc}"`)
  );
  const customUninstallSh = ensureUtf8Bom(tplUninstallSh.replace(/clibm-plugin-template/g, crateName));
  const customUninstallPs1 = ensureUtf8Bom(tplUninstallPs1.replace(/clibm-plugin-template/g, crateName));

  return {
    crateName,
    files: [
      { path: `${crateName}/Cargo.toml`, content: customCargoToml },
      { path: `${crateName}/README.md`, content: customReadme },
      { path: `${crateName}/src/main.rs`, content: customMainRs },
      { path: `${crateName}/install.sh`, content: customInstallSh },
      { path: `${crateName}/install.ps1`, content: customInstallPs1 },
      { path: `${crateName}/uninstall.sh`, content: customUninstallSh },
      { path: `${crateName}/uninstall.ps1`, content: customUninstallPs1 },
    ],
  };
}

export function downloadCustomPluginTemplateZip(
  rawName: string,
  description: string,
  hooks: string[]
): void {
  const { crateName, files } = buildCustomPluginTemplateFiles(rawName, description, hooks);
  const zipBlob = buildZipArchive(files);
  triggerBlobDownload(zipBlob, `${crateName}-v0.1.0.zip`);
}

/**
 * Downloads the 100% Pure Core-Only CLIBM package (`clibm-core-v0.1.0.zip`),
 * containing strictly `clibm_core` + `clibm` CLI binary with zero external plugins.
 * All files are UTF-8-BOM (`EF BB BF`) encoded.
 */
export function downloadCoreOnlyZip(): void {
  const entries: ZipFileEntry[] = RUST_PROJECT_FILES.filter((f) => f.isCoreOnly).map((f) => ({
    path: f.zipPath.replace(/^clibm\//, 'clibm-core/'),
    content: ensureUtf8Bom(f.content),
  }));
  const zipBlob = buildZipArchive(entries);
  triggerBlobDownload(zipBlob, 'clibm-core-v0.1.0.zip');
}

/**
 * Downloads the complete CLIBM bundle (`clibm-v0.1.0-workspace.zip`),
 * including `clibm_core`, `clibm` CLI, and all standalone plugin crates in `plugins/`.
 * All files are UTF-8-BOM (`EF BB BF`) encoded.
 */
export function downloadRustCargoZip(): void {
  const entries: ZipFileEntry[] = RUST_PROJECT_FILES.map((f) => ({
    path: f.zipPath,
    content: ensureUtf8Bom(f.content),
  }));
  const zipBlob = buildZipArchive(entries);
  triggerBlobDownload(zipBlob, 'clibm-v0.1.0-workspace.zip');
}

export function downloadPluginZip(pluginId: string): void {
  const pkg = OFFICIAL_PLUGIN_PACKAGES.find((p) => p.id === pluginId);
  if (!pkg) return;
  const zipBlob = buildZipArchive(pkg.files);
  triggerBlobDownload(zipBlob, pkg.zipFilename);
}

export function downloadStandaloneWebHtml(): void {
  const blob = createUtf8BomBlob(webUiHtml, 'text/html;charset=utf-8');
  triggerBlobDownload(blob, 'clibm-web-standalone.html');
}
