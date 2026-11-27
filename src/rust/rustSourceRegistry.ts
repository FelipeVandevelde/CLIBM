/**
 * Registry of the Rust sources in `rust-clibm/` for the in-browser viewer, and the links to the
 * source ZIP bundles.
 *
 * The file list comes from `import.meta.glob` over `rust-clibm/`, so a new file shows up here
 * without editing this module. The ZIPs themselves are not built in the browser any more:
 * `scripts/build-bundles.sh` builds them from git in CI (`.github/workflows/bundles.yml`), which
 * checks that each one builds and installs, and publishes them on GitHub Releases. The bundles are
 * listed in `rust-clibm/bundles.json`.
 */

import {
  buildZipArchive,
  createUtf8Blob,
  withoutUtf8Bom,
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
    | 'Build Scripts'
    | 'Tests';
  description: string;
  language: 'rust' | 'toml' | 'markdown' | 'bash' | 'powershell' | 'html' | 'json' | 'text';
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
  /** Name of its source ZIP in rust-clibm/bundles.json (core + this plugin, or the plugin alone). */
  bundleName: string;
  /** File name of that ZIP on the bundles-latest release. */
  zipFilename: string;
  /** Plugin-specific installer choice; omitted when install.sh / install.ps1 asks nothing beyond the target directory. */
  installPrompt?: PluginInstallPromptSpec;
  files: ZipFileEntry[];
}

/** Every file under rust-clibm/ except build output, keyed by its path inside rust-clibm/. */
const RUST_SOURCES: Record<string, string> = Object.fromEntries(
  Object.entries(
    import.meta.glob<string>(['../../rust-clibm/**/*', '!../../rust-clibm/**/target/**'], {
      query: '?raw',
      import: 'default',
      eager: true,
    })
  ).map(([path, content]) => [
    path.replace('../../rust-clibm/', ''),
    // LF like the published ZIPs, whatever line endings this checkout uses.
    withoutUtf8Bom(content).replace(/\r\n/g, '\n'),
  ])
);

/** Contents of rust-clibm/<relativePath>. */
function source(relativePath: string): string {
  const content = RUST_SOURCES[relativePath];
  if (content === undefined) throw new Error(`rust-clibm/${relativePath} is missing from the source registry`);
  return content;
}

/** Rolling release that CI refreshes on every push to main. */
export const BUNDLE_BASE_URL = 'https://github.com/FelipeVandevelde/CLIBM/releases/download/bundles-latest';

export function bundleDownloadUrl(bundleName: string): string {
  return `${BUNDLE_BASE_URL}/${bundleName}.zip`;
}

export const BUNDLE_CHECKSUMS_URL = `${BUNDLE_BASE_URL}/SHA256SUMS`;

function countLines(str: string): number {
  return str.split('\n').length;
}

/** Files that ship in the core-only ZIP: everything but plugins/ (except catalog.json) and tests/. */
function isCorePath(relativePath: string): boolean {
  if (relativePath === 'bundles.json') return false;
  if (relativePath.startsWith('tests/')) return false;
  return !relativePath.startsWith('plugins/') || relativePath === 'plugins/catalog.json';
}

function languageOf(relativePath: string): RustProjectFile['language'] {
  const ext = relativePath.slice(relativePath.lastIndexOf('.') + 1).toLowerCase();
  const byExt: Record<string, RustProjectFile['language']> = {
    rs: 'rust',
    toml: 'toml',
    lock: 'toml',
    md: 'markdown',
    sh: 'bash',
    ps1: 'powershell',
    html: 'html',
    json: 'json',
  };
  return byExt[ext] ?? 'text';
}

function categoryOf(relativePath: string): RustProjectFile['category'] {
  if (relativePath.startsWith('tests/') || relativePath.includes('/tests/')) return 'Tests';
  if (relativePath.startsWith('plugins/')) return 'Plugins (TUI & Web)';
  if (/\.(sh|ps1)$/.test(relativePath)) return 'Build Scripts';
  if (relativePath.startsWith('src/')) return 'Core Engine & Versioning';
  return 'Manifest & Docs';
}

function describe(relativePath: string): string {
  if (relativePath.startsWith('plugins/')) {
    const folder = relativePath.split('/')[1];
    return `Part of the ${folder} plugin crate (plugins/${folder}).`;
  }
  if (relativePath.startsWith('tests/')) return 'End-to-end test of the clibm CLI (cargo test).';
  return `rust-clibm/${relativePath}`;
}

/** The core's Cargo.toml as shipped in clibm-core.zip, where the core is the only workspace member. */
export const CORE_ONLY_CARGO_TOML = source('Cargo.toml').replace(
  /^members\s*=\s*\[[\s\S]*?\]/m,
  'members = [\n    ".",\n]'
);

/** Files of plugins/<folder>/, manifest, README and scripts first, then sources. */
function pluginFiles(folder: string): ZipFileEntry[] {
  const prefix = `plugins/${folder}/`;
  const order = ['Cargo.toml', 'README.md', 'install.sh', 'install.ps1', 'uninstall.sh', 'uninstall.ps1'];
  const rank = (rel: string) => (order.includes(rel) ? order.indexOf(rel) : order.length);
  return Object.keys(RUST_SOURCES)
    .filter((p) => p.startsWith(prefix))
    .map((p) => p.slice(prefix.length))
    .sort((a, b) => rank(a) - rank(b) || a.localeCompare(b))
    .map((rel) => ({ path: `${folder}/${rel}`, content: RUST_SOURCES[prefix + rel] }));
}

/** Hand-written notes shown next to known files; any other file gets a generic note. */
const FILE_NOTES: Record<string, { category: RustProjectFile['category']; description: string }> = {
  'Cargo.toml': {
    category: 'Manifest & Docs',
    description:
      'Standalone Cargo package manifest configuring 100% pure clibm_core lib and clibm CLI binary.',
  },
  'src/main.rs': {
    category: 'CLI & Entrypoint',
    description:
      'Cross-platform native CLI binary entrypoint executing commands, Pure Core setup, and standalone plugin execution.',
  },
  'src/cli.rs': {
    category: 'CLI & Entrypoint',
    description:
      'Clap v4 command grammar including init (--with-defaults, --lang, --auto-commit, --install-path) and plugin install (--transport).',
  },
  'src/lib.rs': {
    category: 'CLI & Entrypoint',
    description:
      'Library crate root exposing the UI-agnostic, 100% pure clibm_core modules.',
  },
  'src/canonical.rs': {
    category: 'CAS & Hashing',
    description:
      'RFC 8785 Canonical JSON key-sorting serializer and BLAKE3 256-bit envelope hash calculation.',
  },
  'src/cas.rs': {
    category: 'CAS & Hashing',
    description:
      'Content-Addressable Storage (.clibm/objects/<shard>/<hash>.json), upward ongoing project discovery, and fsck verifier.',
  },
  'src/models.rs': {
    category: 'Core Engine & Versioning',
    description:
      'Immutable payload structs for Workspace, Project, Task, KanbanBoard, TreeSnapshot, Commit, and ClibmConfig (install_profile, language, install_path, mcp_transport).',
  },
  'src/engine.rs': {
    category: 'Core Engine & Versioning',
    description:
      'Headless ClibmEngine coordinating workspaces, projects, tasks, Kanban workflow rules, and portable bundles.',
  },
  'src/versioning.rs': {
    category: 'Core Engine & Versioning',
    description:
      'Git-inspired commit DAG traversal, structural diffing, task history extraction, and 3-way branch merging.',
  },
  'src/index.rs': {
    category: 'Core Engine & Versioning',
    description:
      'Computed secondary indices (.clibm/index/computed.json) and DFS circular dependency detector.',
  },
  'src/plugin.rs': {
    category: 'Core Engine & Versioning',
    description:
      'Pure Core plugin registry (~/.clibm/plugins/registry.json) with UTF-8 scaffolding and embedded user PATH helper templates.',
  },
  'README.md': {
    category: 'Manifest & Docs',
    description:
      'Build instructions, user PATH ledger reference, and CLI guide for running CLIBM and plugins on Linux and Windows.',
  },
  'install.sh': {
    category: 'Build Scripts',
    description:
      'UTF-8 Linux/macOS interactive Core Installation Wizard (Profile, Language, Auto-Commit, Install Path, and User PATH registration with --no-modify-path).',
  },
  'install.ps1': {
    category: 'Build Scripts',
    description:
      'UTF-8 Windows PowerShell interactive Core Installation Wizard (Profile, Language, Auto-Commit, Custom Path, and REG_EXPAND_SZ User PATH registration with -NoModifyPath).',
  },
  'update.sh': {
    category: 'Build Scripts',
    description:
      'UTF-8 Linux/macOS GitHub repository updater script supporting --repo, --branch, and --plugins.',
  },
  'update.ps1': {
    category: 'Build Scripts',
    description:
      'UTF-8 Windows PowerShell GitHub repository updater script supporting -Repo, -Branch, and -Plugins.',
  },
  'uninstall.sh': {
    category: 'Build Scripts',
    description:
      'UTF-8 Linux/macOS uninstaller removing clibm/plugin binaries and CLIBM-added user PATH entries while preserving .clibm data unless --purge is passed.',
  },
  'uninstall.ps1': {
    category: 'Build Scripts',
    description:
      'UTF-8 Windows PowerShell uninstaller removing clibm/plugin binaries and CLIBM-added user PATH entries while preserving .clibm data unless -Purge is passed.',
  },
  'scripts/env-path.sh': {
    category: 'Build Scripts',
    description:
      'Shared POSIX/Bash user PATH helper (~/.clibm/lib/env-path.sh) managing profile marker blocks and ~/.clibm/path-ledger reference counting.',
  },
  'scripts/env-path.ps1': {
    category: 'Build Scripts',
    description:
      'Shared Windows PowerShell user PATH helper (~/.clibm/lib/env-path.ps1) managing REG_EXPAND_SZ HKCU:\\Environment Path and ~/.clibm/path-ledger.',
  },
  'scripts/sync-env-path.ps1': {
    category: 'Build Scripts',
    description:
      'Developer synchronization & drift-verification script (-Check) keeping embedded env-path helpers aligned across all standalone plugin scripts.',
  },
  'plugins/clibm-mcp/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Cargo manifest for the self-hosted Model Context Protocol (MCP) server plugin.',
  },
  'plugins/clibm-mcp/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Modular MCP entry point with compile-time transport selection and dual-transport runtime.',
  },
  'plugins/clibm-mcp/src/config.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Configuration loader and CLI parser for clibm-mcp.',
  },
  'plugins/clibm-mcp/src/error.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standard JSON-RPC 2.0 and MCP error definitions.',
  },
  'plugins/clibm-mcp/src/protocol/mod.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'MCP protocol module declaration.',
  },
  'plugins/clibm-mcp/src/protocol/types.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'JSON-RPC request, response, and protocol version definitions.',
  },
  'plugins/clibm-mcp/src/protocol/schema.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'MCP schema data models for tools, resources, and prompts.',
  },
  'plugins/clibm-mcp/src/backend/mod.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'CLIBM backend bridge module declaration.',
  },
  'plugins/clibm-mcp/src/backend/bridge.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Bridge to the local CLIBM CLI and CAS repository.',
  },
  'plugins/clibm-mcp/src/adapter/mod.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Shared MCP adapter orchestrating registries and request handling.',
  },
  'plugins/clibm-mcp/src/adapter/discovery.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Dynamic plugin tool discovery from registry.json and live reloading monitor.',
  },
  'plugins/clibm-mcp/src/adapter/dispatcher.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Request dispatcher routing JSON-RPC methods to registries.',
  },
  'plugins/clibm-mcp/src/adapter/permissions.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Permission and sandbox policy manager for tools and resources.',
  },
  'plugins/clibm-mcp/src/adapter/tools.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'MCP tool registry with clibm_status, task_list, task_add, board_move, workspace_switch.',
  },
  'plugins/clibm-mcp/src/adapter/validator.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'JSON Schema validator for MCP dynamic plugin tools.',
  },
  'plugins/clibm-mcp/src/adapter/resources.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'MCP resource registry for staged trees, repo config, and Kanban boards.',
  },
  'plugins/clibm-mcp/src/adapter/prompts.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'MCP prompt registry providing guided planning and commit summary templates.',
  },
  'plugins/clibm-mcp/src/transport/mod.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Pluggable transport trait and declarations.',
  },
  'plugins/clibm-mcp/src/transport/stdio.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Zero-overhead STDIO JSON-RPC 2.0 transport for IDEs and CLI agents.',
  },
  'plugins/clibm-mcp/src/transport/http.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Local HTTP JSON-RPC 2.0 server transport using tiny_http.',
  },
  'plugins/clibm-mcp/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Documentation for configuring clibm-mcp transport options (both, stdio, http) and connecting AI agents.',
  },
  'plugins/clibm-mcp/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer for clibm-mcp letting the user choose stdio (default), both, or http transport.',
  },
  'plugins/clibm-mcp/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer for clibm-mcp letting the user choose stdio (default), both, or http.',
  },
  'plugins/clibm-mcp/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller for clibm-mcp with interactive keep/purge config choice.',
  },
  'plugins/clibm-mcp/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller for clibm-mcp.',
  },
  'plugins/clibm-dep-audit/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Cargo manifest for the extracted DAG & Critical Path Auditor plugin.',
  },
  'plugins/clibm-dep-audit/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone DFS circular dependency and critical-path blocker analyzer crate.',
  },
  'plugins/clibm-dep-audit/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Documentation for installing and running clibm-dep-audit from plugins/clibm-dep-audit.',
  },
  'plugins/clibm-dep-audit/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer for clibm-dep-audit.',
  },
  'plugins/clibm-dep-audit/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer for clibm-dep-audit.',
  },
  'plugins/clibm-dep-audit/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller for clibm-dep-audit.',
  },
  'plugins/clibm-dep-audit/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller for clibm-dep-audit.',
  },
  'plugins/clibm-export-md/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Cargo manifest for the extracted Markdown Sprint Exporter plugin.',
  },
  'plugins/clibm-export-md/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Markdown table and sprint report generator over BLAKE3 CAS snapshots.',
  },
  'plugins/clibm-export-md/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Documentation for installing and running clibm-export-md from plugins/clibm-export-md.',
  },
  'plugins/clibm-export-md/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer for clibm-export-md.',
  },
  'plugins/clibm-export-md/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer for clibm-export-md.',
  },
  'plugins/clibm-export-md/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller for clibm-export-md.',
  },
  'plugins/clibm-export-md/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller for clibm-export-md.',
  },
  'plugins/clibm-workflow-lint/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Cargo manifest for the extracted Kanban WIP & Policy Linter plugin.',
  },
  'plugins/clibm-workflow-lint/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Kanban WIP limit, assignee policy, and dependency completion validator crate.',
  },
  'plugins/clibm-workflow-lint/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Documentation for installing and running clibm-workflow-lint from plugins/clibm-workflow-lint.',
  },
  'plugins/clibm-workflow-lint/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer for clibm-workflow-lint.',
  },
  'plugins/clibm-workflow-lint/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer for clibm-workflow-lint.',
  },
  'plugins/clibm-workflow-lint/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller for clibm-workflow-lint.',
  },
  'plugins/clibm-workflow-lint/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller for clibm-workflow-lint.',
  },
  'plugins/clibm-tui/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Cargo manifest for the Ratatui + Crossterm terminal UI plugin (built from its own folder).',
  },
  'plugins/clibm-tui/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Documentation for building clibm-tui in its own folder and running it inside ongoing projects.',
  },
  'plugins/clibm-tui/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Self-contained Ratatui split-pane TUI plugin attaching to ongoing .clibm projects.',
  },
  'plugins/clibm-tui/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer for clibm-tui.',
  },
  'plugins/clibm-tui/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer for clibm-tui.',
  },
  'plugins/clibm-tui/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller for clibm-tui.',
  },
  'plugins/clibm-tui/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller for clibm-tui.',
  },
  'plugins/clibm-web/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Standalone Cargo manifest for the single-binary local HTTP server and embedded web UI plugin.',
  },
  'plugins/clibm-web/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Documentation for building clibm-web in its own folder and running it inside ongoing projects.',
  },
  'plugins/clibm-web/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Self-contained Rust HTTP server binding to 127.0.0.1:7878 over any ongoing .clibm project.',
  },
  'plugins/clibm-web/src/ui.html': {
    category: 'Plugins (TUI & Web)',
    description:
      'Zero-dependency self-hosted HTML/JS application embedded into the clibm-web binary via include_str!.',
  },
  'plugins/clibm-web/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer for clibm-web.',
  },
  'plugins/clibm-web/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer for clibm-web.',
  },
  'plugins/clibm-web/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller for clibm-web.',
  },
  'plugins/clibm-web/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller for clibm-web.',
  },
  'plugins/plugin-template/Cargo.toml': {
    category: 'Plugins (TUI & Web)',
    description:
      'Official starter Cargo manifest for creating new standalone CLIBM plugins.',
  },
  'plugins/plugin-template/src/main.rs': {
    category: 'Plugins (TUI & Web)',
    description:
      'Official starter Rust source implementing CLIBM_PLUGIN_CONTRACT_V1 JSON stdin/stdout and direct project inspection.',
  },
  'plugins/plugin-template/README.md': {
    category: 'Plugins (TUI & Web)',
    description:
      'Developer guide for scaffolding, compiling, installing, and running custom CLIBM plugins.',
  },
  'plugins/plugin-template/install.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Linux installer script for the plugin template.',
  },
  'plugins/plugin-template/install.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 interactive Windows PowerShell installer script for the plugin template.',
  },
  'plugins/plugin-template/uninstall.sh': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Linux uninstaller script for removing the compiled template plugin binary.',
  },
  'plugins/plugin-template/uninstall.ps1': {
    category: 'Plugins (TUI & Web)',
    description:
      'UTF-8 Windows PowerShell uninstaller script for removing the compiled template plugin binary.',
  },
};

export const RUST_PROJECT_FILES: RustProjectFile[] = Object.keys(RUST_SOURCES)
  .sort((a, b) => fileRank(a) - fileRank(b) || a.localeCompare(b))
  .map((relativePath) => {
    const content = RUST_SOURCES[relativePath];
    const note = FILE_NOTES[relativePath];
    return {
      relativePath,
      zipPath: `clibm-workspace/${relativePath}`,
      category: note?.category ?? categoryOf(relativePath),
      description: note?.description ?? describe(relativePath),
      language: languageOf(relativePath),
      content,
      lines: countLines(content),
      isCoreOnly: isCorePath(relativePath),
    };
  });

function fileRank(relativePath: string): number {
  const i = Object.keys(FILE_NOTES).indexOf(relativePath);
  if (i !== -1) return i;
  return isCorePath(relativePath) ? 1000 : 2000;
}

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
      'Modular crate in plugins/clibm-mcp. Features compile-time transport gating (transport-stdio, transport-http) with a decoupled Shared Adapter (ToolRegistry, ResourceRegistry, PromptRegistry, PermissionManager, RequestDispatcher). All install/uninstall scripts are UTF-8 encoded.',
    stack: 'Rust 2021 · MCP 2024-11-05 · Modular Transports (stdio / tiny_http) · UTF-8',
    hooks: ['mcp-stdio', 'mcp-http'],
    installCommand: 'cd plugins/clibm-mcp && ./install.sh --transport both',
    cliCommand: 'clibm plugin run mcp',
    binaryInvoke: 'clibm-mcp',
    bundleName: 'clibm-mcp',
    zipFilename: 'clibm-mcp.zip',
    installPrompt: {
      flagName: '--transport',
      ps1ParamName: '-Transport',
      promptTitle: 'Select MCP Transport Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-mcp, the installer asks which transport to build (stdio is the default, the one AI clients launch):',
      defaultValue: 'stdio',
      options: [
        {
          value: 'stdio',
          label: '1) stdio JSON-RPC 2.0 (Default)',
          detail: 'JSON-RPC over stdin/stdout, what AI clients launch (zero network/HTTP dependencies)',
        },
        {
          value: 'both',
          label: '2) Both Transports',
          detail: 'stdio plus a local HTTP endpoint (127.0.0.1:7879)',
        },
        {
          value: 'http',
          label: '3) Local HTTP Server Only',
          detail: 'Enable only Local HTTP JSON-RPC 2.0 Server on 127.0.0.1:7879',
        },
      ],
    },
    files: pluginFiles('clibm-mcp'),
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
    stack: 'Rust 2021 · DFS Cycle Detector · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8',
    hooks: ['on-task-depend'],
    installCommand: 'cd plugins/clibm-dep-audit && ./install.sh',
    cliCommand: 'clibm plugin run dep-audit',
    binaryInvoke: 'clibm-dep-audit',
    bundleName: 'clibm-dep-audit',
    zipFilename: 'clibm-dep-audit.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select DAG Audit Hook Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-dep-audit, choose whether to run automatically on commits/dependencies or manually:',
      defaultValue: 'on-task-depend',
      options: [
        {
          value: 'on-task-depend',
          label: '1) Automatic Hooks (Default)',
          detail: 'Audit DAG cycles automatically on on-task-depend',
        },
        {
          value: 'manual-run',
          label: '2) Manual Audit Only',
          detail: 'Run only when invoked via clibm plugin run dep-audit',
        },
      ],
    },
    files: pluginFiles('clibm-dep-audit'),
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
    stack: 'Rust 2021 · Canonical JSON Reader · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8',
    hooks: ['on-export'],
    installCommand: 'cd plugins/clibm-export-md && ./install.sh',
    cliCommand: 'clibm plugin run export-md',
    binaryInvoke: 'clibm-export-md',
    bundleName: 'clibm-export-md',
    zipFilename: 'clibm-export-md.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select Markdown Export Hook Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-export-md, choose whether to trigger Markdown export hooks automatically or manually:',
      defaultValue: 'on-export',
      options: [
        {
          value: 'on-export',
          label: '1) Automatic Export Hook (Default)',
          detail: 'Run on-export to keep sprint Markdown reports updated',
        },
        {
          value: 'manual-run',
          label: '2) Manual Export Only',
          detail: 'Run only when invoked via clibm plugin run export-md',
        },
      ],
    },
    files: pluginFiles('clibm-export-md'),
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
      'Standalone Cargo crate in plugins/clibm-workflow-lint. Runs on post-commit (or only on demand) to validate the workflow rules defined in KanbanBoardPayload.',
    stack: 'Rust 2021 · Workflow Policy Engine · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8',
    hooks: ['post-commit'],
    installCommand: 'cd plugins/clibm-workflow-lint && ./install.sh',
    cliCommand: 'clibm plugin run workflow-lint',
    binaryInvoke: 'clibm-workflow-lint',
    bundleName: 'clibm-workflow-lint',
    zipFilename: 'clibm-workflow-lint.zip',
    installPrompt: {
      flagName: '--hooks',
      ps1ParamName: '-Hooks',
      promptTitle: 'Select Workflow Policy Linter Mode (install.sh / install.ps1)',
      promptDescription:
        'When installing clibm-workflow-lint, select whether to enforce WIP policies automatically or on manual runs:',
      defaultValue: 'post-commit',
      options: [
        {
          value: 'post-commit',
          label: '1) Automatic Policy Check (Default)',
          detail: 'Validate WIP limits and assignees on post-commit',
        },
        {
          value: 'manual-run',
          label: '2) Manual Audit Only',
          detail: 'Run only when invoked via clibm plugin run workflow-lint',
        },
      ],
    },
    files: pluginFiles('clibm-workflow-lint'),
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
      'Cargo crate in plugins/clibm-tui that builds against the core library (its ZIP ships the core with it). Install from inside plugins/clibm-tui via ./install.sh (or clibm plugin install .), then run inside any ongoing project via `clibm plugin run tui` or `clibm-tui`.',
    stack: 'Rust 2021 · ratatui 0.29 · crossterm 0.28 · blake3 1.5 · UTF-8',
    hooks: ['interactive-tui'],
    installCommand: 'cd plugins/clibm-tui && ./install.sh',
    cliCommand: 'clibm plugin run tui',
    binaryInvoke: 'clibm-tui',
    bundleName: 'clibm-tui',
    zipFilename: 'clibm-tui.zip',
    // No installPrompt: the TUI always accepts both Vim keys (h/j/k/l) and arrow keys,
    // so install.sh / install.ps1 only ask for the target directory.
    files: pluginFiles('clibm-tui'),
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
    stack: 'Rust 2021 · tiny_http 0.12 · blake3 1.5 · Embedded HTML5 · UTF-8',
    hooks: ['http-bridge'],
    installCommand: 'cd plugins/clibm-web && ./install.sh',
    cliCommand: 'clibm plugin run web -- --port 7878',
    binaryInvoke: 'clibm-web --port 7878',
    bundleName: 'clibm-web',
    zipFilename: 'clibm-web.zip',
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
    files: pluginFiles('clibm-web'),
  },
  {
    id: 'plugin-template',
    name: 'Official Rust Plugin Starter Template',
    crateName: 'clibm-plugin-template',
    version: '0.1.0',
    category: 'Plugin Template',
    summary:
      'Ready-to-compile standalone Rust plugin template implementing CLIBM_PLUGIN_CONTRACT_V1 stdin/stdout hooks, .clibm/ project inspection, and UTF-8 folder-scoped install/uninstall scripts.',
    architectureNotes:
      'Use `clibm plugin new <name>` or customize this template directly to create new CLIBM extensions in under 60 seconds. Includes UTF-8 encoded Cargo.toml, src/main.rs, README.md, install.sh, install.ps1, uninstall.sh, and uninstall.ps1.',
    stack: 'Rust 2021 · serde_json · CLIBM_PLUGIN_CONTRACT_V1 · UTF-8',
    hooks: ['pre-commit', 'on-task-move'],
    installCommand: 'cd plugins/plugin-template && ./install.sh',
    cliCommand: 'clibm plugin new my-plugin',
    binaryInvoke: 'clibm plugin run clibm-plugin-template',
    bundleName: 'clibm-plugin-template',
    zipFilename: 'clibm-plugin-template.zip',
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
          detail: 'Subscribe to pre-commit, post-commit, pre-move, on-task-move, on-task-depend, on-workspace-switch, on-export',
        },
      ],
    },
    files: pluginFiles('plugin-template'),
  },
];

/**
 * The template's Cargo.toml for a crate that lives outside the CLIBM repository: drops the
 * [dev-dependencies] on the core (path "../..", only used by the template's own tests, which are
 * not generated) and makes the crate its own workspace root, like `clibm plugin new` does.
 */
function standaloneTemplateCargoToml(toml: string): string {
  const lines = toml.split('\n');
  const start = lines.findIndex((l) => l.trim() === '[dev-dependencies]');
  if (start !== -1) {
    let first = start;
    while (first > 0 && lines[first - 1].trim().startsWith('#')) first--;
    let end = start + 1;
    while (end < lines.length && !lines[end].trim().startsWith('[')) end++;
    lines.splice(first, end - first);
  }
  return `${lines.join('\n').trimEnd()}\n\n# Its own workspace, so it builds wherever it is unpacked.\n[workspace]\n`;
}

/**
 * Dynamically generates the UTF-8 encoded source and installation/uninstallation
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

  const customCargoToml = withoutUtf8Bom(
    standaloneTemplateCargoToml(source('plugins/plugin-template/Cargo.toml'))
      .replace(/clibm-plugin-template/g, crateName)
      .replace(
        'Official starter template for building standalone CLIBM plugins in Rust',
        cleanDesc.replace(/"/g, "'")
      )
  );

  const customMainRs = withoutUtf8Bom(
    source('plugins/plugin-template/src/main.rs')
      .replace(/clibm-plugin-template/g, crateName)
      .replace(
        '//! CLIBM Official Plugin Starter Template',
        `//! Standalone CLIBM Plugin: ${crateName}\n//! Description: ${cleanDesc}\n//! Subscribed Hooks: ${cleanHooks.join(', ')}`
      )
  );

  const customReadme = withoutUtf8Bom(
    `# ${crateName} (v0.1.0)\n\n${cleanDesc}\n\n- **Contract:** \`CLIBM_PLUGIN_CONTRACT_V1\`\n- **Hooks:** \`${cleanHooks.join('`, `')}\`\n- **Encoding:** \`UTF-8\`\n\n## Quick Install\n\n### Linux, macOS, WSL2\n\`\`\`bash\ncurl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/install.sh | bash\n\`\`\`\n\n### Windows (native, PowerShell)\n\`\`\`powershell\niex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/install.ps1)\n\`\`\`\n\n## Quick Uninstall\n\n### Linux, macOS, WSL2\n\`\`\`bash\ncurl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/uninstall.sh | bash\n\`\`\`\n\n### Windows (native, PowerShell)\n\`\`\`powershell\niex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/${crateName}/uninstall.ps1)\n\`\`\`\n\n## 1. Local Build & Install (From This Plugin Folder)\n\n\`\`\`bash\ncd ${crateName}\n./install.sh\n# Or on Windows PowerShell:\n.\\install.ps1\n\`\`\`\n\n## 2. Run Inside Any Ongoing CLIBM Project\n\n\`\`\`bash\nclibm plugin run ${crateName}\n# Or invoke the binary directly:\n${crateName}\n\`\`\`\n\n## 3. Local Uninstall\n\n\`\`\`bash\n./uninstall.sh\n# Or on Windows PowerShell:\n.\\uninstall.ps1\n\`\`\`\n`
  );

  const safeDesc = cleanDesc.replace(/"/g, "'");
  const hooksCsv = cleanHooks.join(',');
  const hooksPs1 = cleanHooks.map((h) => `"${h}"`).join(', ');

  const customInstallSh = withoutUtf8Bom(
    source('plugins/plugin-template/install.sh')
      .replace(/clibm-plugin-template/g, crateName)
      .replace('TPL_HOOKS="pre-commit,on-task-move"', `TPL_HOOKS="${hooksCsv}"`)
      .replace('PLUGIN_DESC="Standalone CLIBM plugin (${BIN_NAME})"', `PLUGIN_DESC="${safeDesc}"`)
  );
  const customInstallPs1 = withoutUtf8Bom(
    source('plugins/plugin-template/install.ps1')
      .replace(/clibm-plugin-template/g, crateName)
      .replace('"default-hooks" { @("pre-commit", "on-task-move") }', `"default-hooks" { @(${hooksPs1}) }`)
      .replace('$PluginDesc = "Standalone CLIBM plugin ($BinName)"', `$PluginDesc = "${safeDesc}"`)
  );
  const customUninstallSh = withoutUtf8Bom(source('plugins/plugin-template/uninstall.sh').replace(/clibm-plugin-template/g, crateName));
  const customUninstallPs1 = withoutUtf8Bom(source('plugins/plugin-template/uninstall.ps1').replace(/clibm-plugin-template/g, crateName));

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

/** Starts the browser download of a file published on the bundles-latest release. */
function downloadPublishedFile(url: string): void {
  const link = document.createElement('a');
  link.href = url;
  link.rel = 'noopener';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/** Core CLI only (`clibm-core.zip`): the clibm CLI and the clibm_core library, no plugins. */
export function downloadCoreOnlyZip(): void {
  downloadPublishedFile(bundleDownloadUrl('clibm-core'));
}

/** Full Cargo workspace (`clibm-workspace.zip`): core, every official plugin and the tests. */
export function downloadRustCargoZip(): void {
  downloadPublishedFile(bundleDownloadUrl('clibm-workspace'));
}

/** A plugin's ZIP: the core plus that plugin (`clibm-mcp.zip` holds the MCP server alone). */
export function downloadPluginZip(pluginId: string): void {
  const pkg = OFFICIAL_PLUGIN_PACKAGES.find((p) => p.id === pluginId);
  if (!pkg) return;
  downloadPublishedFile(bundleDownloadUrl(pkg.bundleName));
}

export function downloadStandaloneWebHtml(): void {
  const blob = createUtf8Blob(source('plugins/clibm-web/src/ui.html'), 'text/html;charset=utf-8');
  triggerBlobDownload(blob, 'clibm-web-standalone.html');
}
