import React, { useState, useMemo } from 'react';
import {
  Download,
  Code2,
  Play,
  Package,
  Terminal,
  Globe,
  Check,
  Copy,
  Plus,
  Wrench,
  X,
  FileCode2,
} from 'lucide-react';
import {
  OFFICIAL_PLUGIN_PACKAGES,
  buildCustomPluginTemplateFiles,
  downloadCoreOnlyZip,
  downloadCustomPluginTemplateZip,
  downloadPluginZip,
  downloadRustCargoZip,
  DownloadablePluginPackage,
} from '../rust/rustSourceRegistry';
import { ClibmRepositoryState } from '../core/types';
import { generateAiTeachingDocs } from '../core/clibmCore';
import {
  stripUtf8BomForDisplay,
  triggerUtf8BomFileDownload,
} from '../rust/zipBuilder';

interface PluginMarketplaceViewProps {
  repoState: ClibmRepositoryState;
  onRunCommand: (cmd: string) => void;
  onOpenTuiOrWeb: (mode: 'tui' | 'web') => void;
}

const AVAILABLE_HOOKS = [
  'pre-commit',
  'post-commit',
  'on-task-move',
  'on-workspace-switch',
  'on-export',
] as const;

export const PluginMarketplaceView: React.FC<PluginMarketplaceViewProps> = ({
  repoState,
  onRunCommand,
  onOpenTuiOrWeb,
}) => {
  const [categoryFilter, setCategoryFilter] = useState<string>('all');
  const [inspectedPlugin, setInspectedPlugin] = useState<DownloadablePluginPackage>(
    OFFICIAL_PLUGIN_PACKAGES[0]
  );
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);

  // Interactive Plugin Installer Modal State (shows install.sh / install.ps1 choices when user clicks Install)
  const [installingPlugin, setInstallingPlugin] =
    useState<DownloadablePluginPackage | null>(null);
  const [installChoiceValue, setInstallChoiceValue] = useState<string>('both');
  const [installTargetDir, setInstallTargetDir] = useState<string>(
    repoState.config.install_path || '~/.local/bin'
  );
  const [installScriptTab, setInstallScriptTab] = useState<
    'install.sh' | 'install.ps1' | 'uninstall.sh' | 'uninstall.ps1'
  >('install.sh');

  // Plugin Template Studio state
  const [customPluginName, setCustomPluginName] = useState('sprint-velocity');
  const [customPluginDesc, setCustomPluginDesc] = useState(
    'Computes sprint throughput and cycle time metrics across workspaces'
  );
  const [selectedHooks, setSelectedHooks] = useState<string[]>([
    'pre-commit',
    'on-task-move',
  ]);
  const [templateFileIdx, setTemplateFileIdx] = useState(0);

  const filteredPlugins =
    categoryFilter === 'all'
      ? OFFICIAL_PLUGIN_PACKAGES
      : OFFICIAL_PLUGIN_PACKAGES.filter((p) => p.category === categoryFilter);

  const generatedTemplate = useMemo(
    () => buildCustomPluginTemplateFiles(customPluginName, customPluginDesc, selectedHooks),
    [customPluginName, customPluginDesc, selectedHooks]
  );

  const activeTemplateFile =
    generatedTemplate.files[templateFileIdx] || generatedTemplate.files[0];

  const installedPluginNames = useMemo(
    () => new Set(repoState.plugins.map((p) => p.name)),
    [repoState.plugins]
  );

  const handleOpenInstallWizard = (pkg: DownloadablePluginPackage) => {
    setInstallingPlugin(pkg);
    if (pkg.id === 'clibm-mcp' && repoState.config.mcp_transport) {
      setInstallChoiceValue(repoState.config.mcp_transport);
    } else {
      setInstallChoiceValue(pkg.installPrompt.defaultValue);
    }
    setInstallTargetDir(repoState.config.install_path || '~/.local/bin');
    setInstallScriptTab('install.sh');
  };

  const handleConfirmPluginInstall = () => {
    if (!installingPlugin) return;
    if (installingPlugin.id === 'clibm-mcp') {
      onRunCommand(
        `clibm plugin install plugins/clibm-mcp --transport ${installChoiceValue}`
      );
    } else {
      onRunCommand(
        `clibm plugin install plugins/${installingPlugin.crateName}`
      );
    }
    setInstallingPlugin(null);
  };

  const handleToggleHook = (hook: string) => {
    setSelectedHooks((prev) =>
      prev.includes(hook) ? prev.filter((h) => h !== hook) : [...prev, hook]
    );
  };

  const handleScaffoldInRepo = (e: React.FormEvent) => {
    e.preventDefault();
    const hooksArg =
      selectedHooks.length > 0 ? ` --hooks ${selectedHooks.join(',')}` : '';
    const descArg = customPluginDesc.trim()
      ? ` -d "${customPluginDesc.trim().replace(/"/g, "'")}"`
      : '';
    onRunCommand(
      `clibm plugin new ${customPluginName.trim() || 'custom-plugin'}${descArg}${hooksArg}`
    );
  };

  const handleCopy = (id: string, text: string) => {
    navigator.clipboard.writeText(stripUtf8BomForDisplay(text));
    setCopiedCmd(id);
    setTimeout(() => setCopiedCmd(null), 1600);
  };

  const activeFile =
    inspectedPlugin.files[selectedFileIndex] || inspectedPlugin.files[0];

  // Resolve the active install/uninstall script inside the Interactive Install Modal
  const activeModalScriptContent = useMemo(() => {
    if (!installingPlugin) return '';
    const match = installingPlugin.files.find((f) =>
      f.path.endsWith(`/${installScriptTab}`)
    );
    return match ? match.content : installingPlugin.files[0]?.content || '';
  }, [installingPlugin, installScriptTab]);

  // Live AI doc preview when clibm-docs is opened in the Install Wizard
  const liveAiDocPreview = useMemo(() => {
    if (!installingPlugin || installingPlugin.id !== 'clibm-docs') return '';
    const tabMap: Record<string, 'agents' | 'claude' | 'cursorrules' | 'mcp-json'> = {
      all: 'agents',
      agents: 'agents',
      claude: 'claude',
      cursorrules: 'cursorrules',
    };
    return generateAiTeachingDocs(repoState, tabMap[installChoiceValue] || 'agents');
  }, [installingPlugin, installChoiceValue, repoState]);

  return (
    <div className="space-y-6">
      {/* Top Header: 100% Pure Core Status + Standalone Plugins Folder */}
      <div className="border border-zinc-800 bg-zinc-900/70 rounded-lg p-6">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-2 max-w-3xl">
            <div className="flex flex-wrap items-center gap-2 text-xs font-mono text-zinc-300">
              <span className="font-semibold text-white">100% PURE CORE ARCHITECTURE</span>
              <span aria-hidden="true">·</span>
              <span>Core Profile: {repoState.config.install_profile || 'pure'}</span>
              <span aria-hidden="true">·</span>
              <span>Installed Plugins: {repoState.plugins.length}</span>
              <span aria-hidden="true">·</span>
              <span>Scripts &amp; Archives: UTF-8-BOM (EF BB BF)</span>
            </div>
            <h2 className="text-xl font-bold tracking-tight text-white">
              Standalone Plugins Folder (<code className="font-mono text-zinc-200">plugins/*</code>)
            </h2>
            <p className="text-sm text-zinc-300 leading-relaxed">
              The <strong className="text-white">clibm_core</strong> engine starts with{' '}
              <strong className="text-white">0 built-in plugins</strong>. All plugins—including{' '}
              <code className="font-mono text-zinc-200">clibm-mcp</code>,{' '}
              <code className="font-mono text-zinc-200">clibm-docs</code>,{' '}
              <code className="font-mono text-zinc-200">clibm-dep-audit</code>,{' '}
              <code className="font-mono text-zinc-200">clibm-export-md</code>,{' '}
              <code className="font-mono text-zinc-200">clibm-workflow-lint</code>,{' '}
              <code className="font-mono text-zinc-200">clibm-tui</code>,{' '}
              <code className="font-mono text-zinc-200">clibm-web</code>, and{' '}
              <code className="font-mono text-zinc-200">plugin-template</code>—live in{' '}
              <code className="font-mono text-zinc-200">plugins/</code> with UTF-8-BOM encoded{' '}
              <code className="font-mono text-zinc-200">install.sh</code>,{' '}
              <code className="font-mono text-zinc-200">install.ps1</code>,{' '}
              <code className="font-mono text-zinc-200">uninstall.sh</code>, and{' '}
              <code className="font-mono text-zinc-200">uninstall.ps1</code> scripts.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2.5 shrink-0">
            <button
              type="button"
              onClick={() => onRunCommand('clibm plugin install --defaults')}
              className="px-3.5 py-2.5 text-xs font-mono font-semibold bg-zinc-950 hover:bg-zinc-800 text-zinc-100 border border-zinc-700 rounded-lg transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title="Install clibm-dep-audit, clibm-export-md, and clibm-workflow-lint from plugins/"
            >
              Install 3 Default Plugins
            </button>
            <button
              type="button"
              onClick={downloadCoreOnlyZip}
              className="px-4 py-2.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded-lg flex items-center gap-2 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
            >
              <Download className="w-4 h-4" />
              <span>Download Pure Core CLI (.zip)</span>
            </button>
            <button
              type="button"
              onClick={downloadRustCargoZip}
              className="px-4 py-2.5 text-xs font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-100 border border-zinc-700 rounded-lg flex items-center gap-2 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
            >
              <Download className="w-4 h-4 text-zinc-300" />
              <span>Download Core + All Plugins (.zip)</span>
            </button>
          </div>
        </div>

        {/* Category Filter Tabs */}
        <div className="mt-6 pt-5 border-t border-zinc-800 flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-1.5 p-1 bg-zinc-950 border border-zinc-800 rounded-lg flex-wrap">
            {(
              [
                'all',
                'AI & MCP',
                'Analysis & Audit',
                'Export & Interop',
                'Workflow Policy',
                'Interface Plugin',
                'Plugin Template',
              ] as const
            ).map((cat) => (
              <button
                key={cat}
                type="button"
                onClick={() => setCategoryFilter(cat)}
                className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                  categoryFilter === cat
                    ? 'bg-zinc-100 text-zinc-950 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                }`}
              >
                {cat === 'all' ? `All Plugins (${OFFICIAL_PLUGIN_PACKAGES.length})` : cat}
              </button>
            ))}
          </div>

          <span className="text-xs font-mono text-zinc-400">
            Active in repo:{' '}
            {repoState.plugins.length === 0
              ? 'None (Pure Core)'
              : repoState.plugins.map((p) => p.name).join(' · ')}
          </span>
        </div>
      </div>

      {/* Uniform Plugin Cards Grid (All 8 Plugins Including clibm-mcp & clibm-docs Use Identical Card Layout) */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
        {filteredPlugins.map((pkg) => {
          const isInspected = inspectedPlugin.id === pkg.id;
          const isInstalled = installedPluginNames.has(pkg.crateName);
          return (
            <div
              key={pkg.id}
              className={`border rounded-lg p-5 flex flex-col justify-between gap-4 transition-colors ${
                isInspected
                  ? 'border-zinc-300 bg-zinc-900/90'
                  : 'border-zinc-800 bg-zinc-900/50 hover:border-zinc-700'
              }`}
            >
              <div className="space-y-3">
                {/* Unboxed Metadata Header */}
                <div className="flex items-center justify-between gap-2 text-xs font-mono text-zinc-400">
                  <div className="flex items-center gap-1.5">
                    <Package className="w-3.5 h-3.5 text-zinc-200" />
                    <span className="text-zinc-100 font-semibold">{pkg.crateName}</span>
                    <span aria-hidden="true">·</span>
                    <span>v{pkg.version}</span>
                    <span aria-hidden="true">·</span>
                    <span>UTF-8-BOM</span>
                  </div>
                  <span className={isInstalled ? 'text-white font-bold underline' : 'text-zinc-400'}>
                    {isInstalled ? 'INSTALLED' : pkg.category}
                  </span>
                </div>

                <h3 className="text-base font-semibold text-white">{pkg.name}</h3>
                <p className="text-xs text-zinc-300 leading-relaxed">{pkg.summary}</p>

                <div className="pt-2 border-t border-zinc-800/80 space-y-1.5 text-[11px] font-mono text-zinc-400">
                  <div>Stack: {pkg.stack}</div>
                  <div>Hooks: {pkg.hooks.join(' · ')}</div>
                </div>

                {/* Folder-Scoped Install + Ongoing-Project Run Commands */}
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between gap-2 bg-zinc-950 border border-zinc-800 px-3 py-1.5 rounded font-mono text-[11px]">
                    <div className="truncate">
                      <span className="text-zinc-500 mr-1.5">[1. Folder Install]</span>
                      <code className="text-zinc-200">$ {pkg.installCommand}</code>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleCopy(`${pkg.id}-install`, pkg.installCommand)}
                      className="text-zinc-400 hover:text-white shrink-0"
                      title="Copy folder install command"
                    >
                      {copiedCmd === `${pkg.id}-install` ? (
                        <Check className="w-3.5 h-3.5 text-white" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                    </button>
                  </div>

                  <div className="flex items-center justify-between gap-2 bg-zinc-950 border border-zinc-800 px-3 py-1.5 rounded font-mono text-[11px]">
                    <div className="truncate">
                      <span className="text-zinc-500 mr-1.5">[2. Ongoing Project]</span>
                      <code className="text-zinc-100">$ {pkg.cliCommand}</code>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleCopy(pkg.id, pkg.cliCommand)}
                      className="text-zinc-400 hover:text-white shrink-0"
                      title="Copy ongoing project run command"
                    >
                      {copiedCmd === pkg.id ? (
                        <Check className="w-3.5 h-3.5 text-white" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                    </button>
                  </div>
                </div>
              </div>

              {/* Uniform Card Action Buttons */}
              <div className="pt-3 border-t border-zinc-800 flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  onClick={() => handleOpenInstallWizard(pkg)}
                  className="px-3 py-2 text-xs font-mono font-bold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center justify-center gap-1 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                  title="Open install.sh / install.ps1 interactive options to install this plugin"
                >
                  <span>{isInstalled ? 'Reinstall' : 'Install'}</span>
                </button>

                <button
                  type="button"
                  onClick={() => downloadPluginZip(pkg.id)}
                  className="px-3 py-2 text-xs font-medium bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 rounded flex items-center justify-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                  title="Download UTF-8-BOM encoded plugin crate .zip"
                >
                  <Download className="w-3.5 h-3.5" />
                  <span>.zip</span>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setInspectedPlugin(pkg);
                    setSelectedFileIndex(0);
                  }}
                  className="px-3 py-2 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                >
                  <Code2 className="w-3.5 h-3.5 text-zinc-300" />
                  <span>Source</span>
                </button>

                {pkg.id === 'clibm-tui' ? (
                  <button
                    type="button"
                    onClick={() => onOpenTuiOrWeb('tui')}
                    className="px-3 py-2 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-white border border-zinc-600 rounded flex items-center gap-1 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    <Terminal className="w-3.5 h-3.5" />
                    <span>TUI</span>
                  </button>
                ) : pkg.id === 'clibm-web' ? (
                  <button
                    type="button"
                    onClick={() => onOpenTuiOrWeb('web')}
                    className="px-3 py-2 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-white border border-zinc-600 rounded flex items-center gap-1 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    <Globe className="w-3.5 h-3.5" />
                    <span>Web</span>
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => onRunCommand(pkg.cliCommand)}
                    className="px-3 py-2 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-white border border-zinc-600 rounded flex items-center gap-1 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    <Play className="w-3.5 h-3.5" />
                    <span>Run</span>
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Interactive Plugin Install File Modal (Opens when clicking "Install" on any plugin card) */}
      {installingPlugin && (
        <div className="fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-4 overflow-y-auto">
          <div className="w-full max-w-4xl border border-zinc-700 bg-zinc-950 rounded-xl shadow-2xl overflow-hidden my-8">
            {/* Modal Header */}
            <div className="px-6 py-4 bg-zinc-900 border-b border-zinc-800 flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
                  <FileCode2 className="w-4 h-4 text-white" />
                  <span className="font-bold text-white">
                    plugins/{installingPlugin.crateName}/{installScriptTab}
                  </span>
                  <span aria-hidden="true">·</span>
                  <span>Encoding: UTF-8 with BOM (EF BB BF)</span>
                </div>
                <h3 className="text-base font-bold text-white">
                  Install {installingPlugin.name} — Interactive Installer Choices
                </h3>
              </div>

              <button
                type="button"
                onClick={() => setInstallingPlugin(null)}
                className="p-1.5 text-zinc-400 hover:text-white bg-zinc-950 border border-zinc-800 rounded-lg cursor-pointer"
                title="Close installer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-6 space-y-5 max-h-[80vh] overflow-y-auto">
              {/* Step 1: Plugin-Specific Choices from install.sh / install.ps1 */}
              <div className="space-y-3">
                <div className="space-y-1">
                  <div className="text-xs font-mono font-bold text-white">
                    [1/2] {installingPlugin.installPrompt.promptTitle}
                  </div>
                  <p className="text-xs text-zinc-400">
                    {installingPlugin.installPrompt.promptDescription}
                  </p>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
                  {installingPlugin.installPrompt.options.map((opt) => {
                    const selected = installChoiceValue === opt.value;
                    return (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() => setInstallChoiceValue(opt.value)}
                        className={`p-3 rounded-lg border text-left transition-colors cursor-pointer ${
                          selected
                            ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                            : 'bg-zinc-900/70 text-zinc-200 border-zinc-800 hover:border-zinc-700'
                        }`}
                      >
                        <div className="text-xs font-mono font-bold">{opt.label}</div>
                        <div
                          className={`text-[11px] mt-1 leading-snug ${
                            selected ? 'text-zinc-800' : 'text-zinc-400'
                          }`}
                        >
                          {opt.detail}
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Step 2: Target Installation Directory */}
              <div className="grid grid-cols-1 md:grid-cols-12 gap-4 items-end">
                <div className="md:col-span-6">
                  <label className="block text-xs font-mono font-bold text-white mb-1">
                    [2/2] Installation Target Directory (--install-dir / -InstallDir)
                  </label>
                  <input
                    type="text"
                    value={installTargetDir}
                    onChange={(e) => setInstallTargetDir(e.target.value)}
                    className="w-full px-3 py-2 text-xs font-mono bg-zinc-900 border border-zinc-800 rounded-lg text-white focus:outline-none focus:border-zinc-500"
                    placeholder="~/.local/bin"
                  />
                </div>
                <div className="md:col-span-6 bg-zinc-900 border border-zinc-800 rounded-lg px-3.5 py-2 font-mono text-[11px] space-y-0.5">
                  <div className="text-zinc-400">Generated Terminal Command (Linux / Windows):</div>
                  <div className="text-zinc-100 truncate">
                    $ cd plugins/{installingPlugin.crateName} &amp;&amp; ./install.sh{' '}
                    {installingPlugin.installPrompt.flagName} {installChoiceValue} --install-dir &quot;
                    {installTargetDir}&quot;
                  </div>
                </div>
              </div>

              {/* Optional Live AI Teaching Docs Preview when installing clibm-docs */}
              {installingPlugin.id === 'clibm-docs' && liveAiDocPreview && (
                <div className="border border-zinc-800 bg-zinc-900/50 rounded-lg p-3.5 space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-mono font-semibold text-white">
                      Preview Generated AI Context Output ({installChoiceValue}) · UTF-8-BOM
                    </span>
                    <button
                      type="button"
                      onClick={() =>
                        triggerUtf8BomFileDownload(
                          liveAiDocPreview,
                          installChoiceValue === 'claude'
                            ? 'CLAUDE.md'
                            : installChoiceValue === 'cursorrules'
                            ? '.cursorrules'
                            : 'AGENTS.md'
                        )
                      }
                      className="px-2.5 py-1 text-[11px] font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold rounded flex items-center gap-1 cursor-pointer"
                    >
                      <Download className="w-3 h-3" />
                      <span>Download AI Doc (UTF-8-BOM)</span>
                    </button>
                  </div>
                  <pre className="p-2.5 bg-zinc-950 border border-zinc-800 rounded text-[11px] font-mono text-zinc-300 max-h-36 overflow-y-auto">
                    {liveAiDocPreview}
                  </pre>
                </div>
              )}

              {/* Live UTF-8-BOM Install / Uninstall Script File Inspector */}
              <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg overflow-hidden">
                <div className="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-1.5 flex-wrap">
                    {(
                      ['install.sh', 'install.ps1', 'uninstall.sh', 'uninstall.ps1'] as const
                    ).map((scriptName) => (
                      <button
                        key={scriptName}
                        type="button"
                        onClick={() => setInstallScriptTab(scriptName)}
                        className={`px-2.5 py-1 text-xs font-mono rounded transition-colors cursor-pointer ${
                          installScriptTab === scriptName
                            ? 'bg-zinc-100 text-zinc-950 font-bold'
                            : 'bg-zinc-950 text-zinc-300 border border-zinc-800 hover:text-white'
                        }`}
                      >
                        {scriptName}
                      </button>
                    ))}
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={() => handleCopy('modal-script', activeModalScriptContent)}
                      className="px-2.5 py-1 text-xs font-mono bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 rounded flex items-center gap-1 cursor-pointer"
                    >
                      {copiedCmd === 'modal-script' ? (
                        <Check className="w-3.5 h-3.5 text-white" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                      <span>Copy</span>
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        triggerUtf8BomFileDownload(
                          activeModalScriptContent,
                          `${installingPlugin.crateName}-${installScriptTab}`
                        )
                      }
                      className="px-2.5 py-1 text-xs font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold rounded flex items-center gap-1 cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Download {installScriptTab} (UTF-8-BOM)</span>
                    </button>
                  </div>
                </div>

                <pre className="p-4 text-xs font-mono text-zinc-200 overflow-x-auto max-h-56 leading-relaxed bg-zinc-950">
                  {stripUtf8BomForDisplay(activeModalScriptContent)}
                </pre>
              </div>
            </div>

            {/* Modal Footer */}
            <div className="px-6 py-4 bg-zinc-900 border-t border-zinc-800 flex flex-wrap items-center justify-between gap-3">
              <div className="text-xs font-mono text-zinc-400">
                All installation &amp; uninstallation files are encoded in{' '}
                <strong className="text-zinc-200">UTF-8 with BOM (EF BB BF)</strong>.
              </div>
              <div className="flex items-center gap-2.5">
                <button
                  type="button"
                  onClick={() => setInstallingPlugin(null)}
                  className="px-3.5 py-2 text-xs font-mono bg-zinc-950 hover:bg-zinc-800 text-zinc-300 border border-zinc-700 rounded-lg cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleConfirmPluginInstall}
                  className="px-4 py-2 text-xs font-mono font-bold bg-zinc-100 hover:bg-white text-zinc-950 rounded-lg flex items-center gap-1.5 cursor-pointer"
                >
                  <Check className="w-4 h-4" />
                  <span>
                    Confirm &amp; Install {installingPlugin.crateName} ({installChoiceValue})
                  </span>
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Interactive Plugin Template Scaffolder (`clibm plugin new` / `plugins/plugin-template`) */}
      <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-6 space-y-5">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 border-b border-zinc-800 pb-4">
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
              <Wrench className="w-3.5 h-3.5 text-zinc-100" />
              <span className="font-semibold text-white">
                New Plugin Template Scaffolder (plugins/plugin-template)
              </span>
              <span aria-hidden="true">·</span>
              <span>Crate: {generatedTemplate.crateName}</span>
              <span aria-hidden="true">·</span>
              <span>UTF-8-BOM (EF BB BF)</span>
            </div>
            <h3 className="text-base font-bold text-white">
              Create a Standalone Rust Plugin from the Official Template
            </h3>
            <p className="text-xs text-zinc-400">
              Configure your plugin name, description, and lifecycle hooks below. The template
              automatically generates a folder-isolated Rust crate with UTF-8-BOM encoded{' '}
              <code className="font-mono text-zinc-200">Cargo.toml</code>,{' '}
              <code className="font-mono text-zinc-200">src/main.rs</code>,{' '}
              <code className="font-mono text-zinc-200">README.md</code>,{' '}
              <code className="font-mono text-zinc-200">install.sh</code>,{' '}
              <code className="font-mono text-zinc-200">install.ps1</code>,{' '}
              <code className="font-mono text-zinc-200">uninstall.sh</code>, and{' '}
              <code className="font-mono text-zinc-200">uninstall.ps1</code>.
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2.5 shrink-0">
            <button
              type="button"
              onClick={() =>
                downloadCustomPluginTemplateZip(
                  customPluginName,
                  customPluginDesc,
                  selectedHooks
                )
              }
              className="px-4 py-2 text-xs font-mono font-bold bg-zinc-100 hover:bg-white text-zinc-950 rounded-lg flex items-center gap-1.5 transition-colors cursor-pointer whitespace-nowrap shrink-0"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Download {generatedTemplate.crateName}-v0.1.0.zip</span>
            </button>
            <button
              type="button"
              onClick={() => downloadPluginZip('plugin-template')}
              className="px-3.5 py-2 text-xs font-mono bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 rounded-lg flex items-center gap-1.5 transition-colors cursor-pointer whitespace-nowrap shrink-0"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Base Template (.zip)</span>
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
          {/* Left 5 cols: Scaffolder Configuration Form */}
          <form onSubmit={handleScaffoldInRepo} className="lg:col-span-5 space-y-4">
            <div>
              <label className="block text-xs font-mono text-zinc-300 mb-1">
                Plugin Name (auto-prefixed with <code className="text-zinc-100">clibm-</code>)
              </label>
              <input
                type="text"
                value={customPluginName}
                onChange={(e) => setCustomPluginName(e.target.value)}
                placeholder="e.g. sprint-velocity or slack-webhook"
                className="w-full px-3 py-2 text-xs font-mono bg-zinc-950 border border-zinc-800 rounded-lg text-white focus:outline-none focus:border-zinc-500"
              />
            </div>

            <div>
              <label className="block text-xs font-mono text-zinc-300 mb-1">
                Crate Description
              </label>
              <input
                type="text"
                value={customPluginDesc}
                onChange={(e) => setCustomPluginDesc(e.target.value)}
                placeholder="Describe what your plugin does..."
                className="w-full px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded-lg text-white focus:outline-none focus:border-zinc-500"
              />
            </div>

            <div>
              <label className="block text-xs font-mono text-zinc-300 mb-1.5">
                Subscribed Contract Hooks (<code className="text-zinc-100">CLIBM_PLUGIN_CONTRACT_V1</code>)
              </label>
              <div className="flex flex-wrap gap-1.5">
                {AVAILABLE_HOOKS.map((hook) => {
                  const active = selectedHooks.includes(hook);
                  return (
                    <button
                      key={hook}
                      type="button"
                      onClick={() => handleToggleHook(hook)}
                      className={`px-2.5 py-1 rounded text-xs font-mono border transition-colors cursor-pointer whitespace-nowrap shrink-0 ${
                        active
                          ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                          : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:text-zinc-200'
                      }`}
                    >
                      {hook}
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="bg-zinc-950 border border-zinc-800 rounded-lg p-3 space-y-1.5 font-mono text-[11px]">
              <div className="text-zinc-400">Generated CLI Scaffold Command:</div>
              <div className="text-zinc-100 break-all">
                $ clibm plugin new {customPluginName.trim() || 'custom-plugin'} --hooks{' '}
                {selectedHooks.join(',') || 'manual-run'}
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-2.5 pt-1">
              <button
                type="submit"
                className="px-4 py-2 text-xs font-mono font-bold bg-zinc-100 hover:bg-white text-zinc-950 rounded-lg flex items-center gap-1.5 transition-colors cursor-pointer whitespace-nowrap shrink-0"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Scaffold &amp; Register via CLI</span>
              </button>
              <button
                type="button"
                onClick={() => onRunCommand(`clibm plugin run ${generatedTemplate.crateName}`)}
                className="px-3.5 py-2 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg flex items-center gap-1.5 transition-colors cursor-pointer whitespace-nowrap shrink-0"
              >
                <Play className="w-3.5 h-3.5" />
                <span>Test Run Plugin</span>
              </button>
            </div>
          </form>

          {/* Right 7 cols: Live Generated Template Files Preview */}
          <div className="lg:col-span-7 border border-zinc-800 bg-zinc-950 rounded-lg overflow-hidden">
            <div className="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 flex-wrap">
                {generatedTemplate.files.map((file, idx) => (
                  <button
                    key={file.path}
                    type="button"
                    onClick={() => setTemplateFileIdx(idx)}
                    className={`px-2.5 py-1 text-[11px] font-mono rounded transition-colors cursor-pointer whitespace-nowrap shrink-0 ${
                      templateFileIdx === idx
                        ? 'bg-zinc-100 text-zinc-950 font-bold'
                        : 'bg-zinc-800 text-zinc-300 hover:text-white'
                    }`}
                  >
                    {file.path.replace(`${generatedTemplate.crateName}/`, '')}
                  </button>
                ))}
              </div>
              {activeTemplateFile && (
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => handleCopy('tpl-file', activeTemplateFile.content)}
                    className="text-xs font-mono text-zinc-300 hover:text-white flex items-center gap-1 cursor-pointer"
                  >
                    {copiedCmd === 'tpl-file' ? (
                      <Check className="w-3.5 h-3.5 text-white" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                    <span>Copy</span>
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      triggerUtf8BomFileDownload(
                        activeTemplateFile.content,
                        activeTemplateFile.path.split('/').pop() || 'file.txt'
                      )
                    }
                    className="px-2 py-1 text-[11px] font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold rounded flex items-center gap-1 cursor-pointer"
                  >
                    <Download className="w-3 h-3" />
                    <span>UTF-8-BOM</span>
                  </button>
                </div>
              )}
            </div>

            {activeTemplateFile && (
              <pre className="p-4 text-xs font-mono text-zinc-200 overflow-x-auto max-h-72 leading-relaxed">
                {stripUtf8BomForDisplay(activeTemplateFile.content)}
              </pre>
            )}
          </div>
        </div>
      </div>

      {/* Selected Plugin Source Code & Architecture Inspector */}
      <div className="border border-zinc-800 bg-zinc-950 rounded-lg overflow-hidden">
        <div className="px-5 py-4 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-4">
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
              <span className="text-white font-semibold">
                Inspecting Plugin Crate: {inspectedPlugin.crateName}
              </span>
              <span aria-hidden="true">·</span>
              <span>Archive: {inspectedPlugin.zipFilename}</span>
              <span aria-hidden="true">·</span>
              <span>Encoding: UTF-8-BOM (EF BB BF)</span>
            </div>
            <p className="text-xs text-zinc-300">{inspectedPlugin.architectureNotes}</p>
          </div>

          <div className="flex items-center gap-2 flex-wrap">
            {inspectedPlugin.files.map((f, idx) => (
              <button
                key={f.path}
                type="button"
                onClick={() => setSelectedFileIndex(idx)}
                className={`px-3 py-1.5 text-xs font-mono rounded transition-colors cursor-pointer ${
                  selectedFileIndex === idx
                    ? 'bg-zinc-100 text-zinc-950 font-semibold'
                    : 'bg-zinc-800 text-zinc-300 hover:text-white'
                }`}
              >
                {f.path}
              </button>
            ))}
            {activeFile && (
              <button
                type="button"
                onClick={() =>
                  triggerUtf8BomFileDownload(
                    activeFile.content,
                    activeFile.path.split('/').pop() || 'file.txt'
                  )
                }
                className="px-3 py-1.5 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-100 border border-zinc-700 rounded flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Download File (UTF-8-BOM)</span>
              </button>
            )}
            <button
              type="button"
              onClick={() => downloadPluginZip(inspectedPlugin.id)}
              className="px-3 py-1.5 text-xs font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Download {inspectedPlugin.zipFilename}</span>
            </button>
          </div>
        </div>

        {activeFile && (
          <div className="p-4 overflow-x-auto max-h-[460px]">
            <pre className="font-mono text-xs text-zinc-200 leading-relaxed tabular-nums">
              {stripUtf8BomForDisplay(activeFile.content)
                .split('\n')
                .map((line, i) => (
                  <div key={i} className="table-row hover:bg-zinc-900/60">
                    <span className="table-cell pr-4 text-right text-zinc-600 select-none w-10">
                      {i + 1}
                    </span>
                    <span className="table-cell whitespace-pre">{line}</span>
                  </div>
                ))}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
};
