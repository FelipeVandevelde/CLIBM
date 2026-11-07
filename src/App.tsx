/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useCallback, useRef, useMemo } from 'react';
import {
  Download,
  Upload,
  RotateCcw,
  Terminal,
  FolderGit2,
  GitBranch,
  Database,
  Kanban,
  BookOpen,
  Package,
  Cpu,
  Sparkles,
  Sliders,
  ArrowRight,
  GitCommit,
} from 'lucide-react';
import {
  executeCliCommand,
  getActiveBranch,
  getStagedTree,
  getTypedPayload,
  loadOrInitRepository,
  resolveHeadCommit,
  saveRepository,
} from './core/clibmCore';
import {
  ClibmRepositoryState,
  TerminalHistoryEntry,
  WorkspacePayload,
} from './core/types';
import { downloadCoreOnlyZip, downloadRustCargoZip } from './rust/rustSourceRegistry';
import { triggerBlobDownload } from './rust/zipBuilder';
import { TerminalPane } from './components/TerminalPane';
import { KanbanAndTasksView } from './components/KanbanAndTasksView';
import { VersioningAndDiffView } from './components/VersioningAndDiffView';
import { CasInspectorView } from './components/CasInspectorView';
import { RustPackageView } from './components/RustPackageView';
import { ShowcaseView } from './components/ShowcaseView';
import { DocumentationView } from './components/DocumentationView';
import { PluginMarketplaceView } from './components/PluginMarketplaceView';
import { TuiAndWebPluginsView } from './components/TuiAndWebPluginsView';
import { ConfigurationView } from './components/ConfigurationView';

type PrimaryNavTab = 'showcase' | 'docs' | 'plugins' | 'tui-web' | 'workbench' | 'config';
type WorkbenchSubTab = 'terminal-kanban' | 'versioning' | 'cas' | 'config' | 'rust-source';

export default function App() {
  const [repoState, setRepoState] = useState<ClibmRepositoryState>(() => loadOrInitRepository());
  const [activeTab, setActiveTab] = useState<PrimaryNavTab>('showcase');
  const [workbenchSubTab, setWorkbenchSubTab] = useState<WorkbenchSubTab>('terminal-kanban');
  const [tuiOrWebMode, setTuiOrWebMode] = useState<'tui' | 'web'>('tui');
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [terminalHistory, setTerminalHistory] = useState<TerminalHistoryEntry[]>(() => {
    const initial = loadOrInitRepository();
    const statusRes = executeCliCommand(initial, 'clibm status');
    const boardRes = executeCliCommand(initial, 'clibm board show');
    return [
      {
        id: 'init-0',
        command: 'clibm status',
        output: statusRes.output,
        status: 'ok',
        timestamp: new Date().toISOString(),
      },
      {
        id: 'init-1',
        command: 'clibm board show',
        output: boardRes.output,
        status: 'ok',
        timestamp: new Date().toISOString(),
      },
    ];
  });

  const handleUpdateStateAndHistory = useCallback(
    (
      nextState: ClibmRepositoryState,
      cmd: string,
      output: string,
      status: 'ok' | 'error' | 'info'
    ) => {
      setRepoState(nextState);
      setTerminalHistory((prev) => [
        ...prev,
        {
          id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
          command: cmd,
          output,
          status,
          timestamp: new Date().toISOString(),
        },
      ]);
    },
    []
  );

  const handleExecuteCommand = useCallback(
    (cmd: string) => {
      const result = executeCliCommand(repoState, cmd);
      handleUpdateStateAndHistory(result.nextState, cmd, result.output, result.status);
    },
    [repoState, handleUpdateStateAndHistory]
  );

  const handleExportBundle = () => {
    const bundleJson = JSON.stringify(repoState, null, 2);
    const blob = new Blob([bundleJson], { type: 'application/json' });
    triggerBlobDownload(blob, 'clibm-repository-bundle.json');
  };

  const handleImportBundle = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const parsed = JSON.parse(String(reader.result)) as ClibmRepositoryState;
        if (parsed && parsed.objects && parsed.stagedTreeHash) {
          setRepoState(parsed);
          saveRepository(parsed);
          setTerminalHistory((prev) => [
            ...prev,
            {
              id: String(Date.now()),
              command: `clibm import ${file.name}`,
              output: `Imported ${Object.keys(parsed.objects).length} BLAKE3 CAS objects from ${file.name}.`,
              status: 'ok',
              timestamp: new Date().toISOString(),
            },
          ]);
        }
      } catch (err) {
        setTerminalHistory((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            command: `clibm import ${file.name}`,
            output: `Failed to parse repository bundle: ${
              err instanceof Error ? err.message : String(err)
            }`,
            status: 'error',
            timestamp: new Date().toISOString(),
          },
        ]);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  const activeBranch = getActiveBranch(repoState);
  const headCommit = resolveHeadCommit(repoState);
  const objectCount = Object.keys(repoState.objects).length;

  const { activeWorkspace, sortedWorkspaces } = useMemo(() => {
    const tree = getStagedTree(repoState);
    const loadedWorkspaces: WorkspacePayload[] = [];
    for (const [, wsHash] of Object.entries(tree.workspaces)) {
      const { payload: ws } = getTypedPayload<WorkspacePayload>(repoState, wsHash, 'workspace');
      loadedWorkspaces.push(ws);
    }
    loadedWorkspaces.sort(
      (a, b) =>
        a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id)
    );
    const currentWs =
      loadedWorkspaces.find((w) => w.id === tree.active_workspace_id) || loadedWorkspaces[0];
    return { activeWorkspace: currentWs, sortedWorkspaces: loadedWorkspaces };
  }, [repoState]);

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col">
      {/* Strict 3-Zone Top Bar Contract (Pure Monochrome) */}
      <header className="flex items-center justify-between gap-8 px-6 py-3.5 border-b border-zinc-800 bg-zinc-950/95 sticky top-0 z-30">
        {/* Zone 1: Single text element wordmark */}
        <a
          href="#showcase"
          onClick={(e) => {
            e.preventDefault();
            setActiveTab('showcase');
          }}
          className="text-base font-bold tracking-tight text-white whitespace-nowrap shrink-0 font-display"
        >
          CLIBM v0.1.0
        </a>

        {/* Zone 2: Concise single-line text navigation links */}
        <nav className="hidden md:flex items-center gap-6 text-xs font-medium text-zinc-400">
          <a
            href="#showcase"
            onClick={(e) => {
              e.preventDefault();
              setActiveTab('showcase');
            }}
            className={`hover:text-white transition-colors whitespace-nowrap shrink-0 pb-0.5 ${
              activeTab === 'showcase' ? 'text-white border-b border-white' : ''
            }`}
          >
            Showcase &amp; Demo
          </a>
          <a
            href="#docs"
            onClick={(e) => {
              e.preventDefault();
              setActiveTab('docs');
            }}
            className={`hover:text-white transition-colors whitespace-nowrap shrink-0 pb-0.5 ${
              activeTab === 'docs' ? 'text-white border-b border-white' : ''
            }`}
          >
            Documentation
          </a>
          <a
            href="#plugins"
            onClick={(e) => {
              e.preventDefault();
              setActiveTab('plugins');
            }}
            className={`hover:text-white transition-colors whitespace-nowrap shrink-0 pb-0.5 ${
              activeTab === 'plugins' ? 'text-white border-b border-white' : ''
            }`}
          >
            Plugins
          </a>
          <a
            href="#tui-web"
            onClick={(e) => {
              e.preventDefault();
              setActiveTab('tui-web');
            }}
            className={`hover:text-white transition-colors whitespace-nowrap shrink-0 pb-0.5 ${
              activeTab === 'tui-web' ? 'text-white border-b border-white' : ''
            }`}
          >
            TUI &amp; Web App
          </a>
          <a
            href="#workbench"
            onClick={(e) => {
              e.preventDefault();
              setActiveTab('workbench');
            }}
            className={`hover:text-white transition-colors whitespace-nowrap shrink-0 pb-0.5 ${
              activeTab === 'workbench' ? 'text-white border-b border-white' : ''
            }`}
          >
            CLI Workbench
          </a>
          <a
            href="#config"
            onClick={(e) => {
              e.preventDefault();
              setActiveTab('config');
            }}
            className={`hover:text-white transition-colors whitespace-nowrap shrink-0 pb-0.5 ${
              activeTab === 'config' ? 'text-white border-b border-white' : ''
            }`}
          >
            Config
          </a>
        </nav>

        {/* Zone 3: 1 primary action (Core-Only Download) */}
        <div className="flex items-center gap-3 shrink-0">
          <button
            type="button"
            onClick={downloadCoreOnlyZip}
            className="px-4 py-2 text-xs font-semibold text-zinc-950 bg-zinc-100 rounded-lg hover:bg-white transition-colors whitespace-nowrap shrink-0 cursor-pointer"
          >
            Download Core CLI (.zip)
          </button>
        </div>
      </header>

      {/* Sub-header Live Repository Context, Workspace Switcher & Auto-Commit Controls */}
      <div className="border-b border-zinc-800/80 bg-zinc-900/40 px-6 py-2.5">
        <div className="max-w-[1440px] mx-auto flex flex-wrap items-center justify-between gap-4">
          {/* Interactive Workspace Switcher & Unboxed Metadata Breadcrumb */}
          <div className="flex flex-wrap items-center gap-2 text-xs font-mono text-zinc-400">
            <div className="flex items-center gap-1.5">
              <span>Workspace:</span>
              <select
                value={activeWorkspace?.id || ''}
                onChange={(e) =>
                  handleExecuteCommand(`clibm workspace switch ${e.target.value}`)
                }
                aria-label="Active Workspace Selector"
                className="bg-zinc-950 text-zinc-100 font-semibold border border-zinc-700 rounded px-2 py-0.5 text-xs font-mono focus:outline-none focus:border-zinc-400 cursor-pointer"
              >
                {sortedWorkspaces.map((ws) => (
                  <option key={ws.id} value={ws.id}>
                    {ws.name} ({ws.id})
                  </option>
                ))}
              </select>
              <button
                type="button"
                onClick={() => handleExecuteCommand('clibm workspace switch')}
                className="px-2 py-0.5 text-[11px] font-mono text-zinc-200 bg-zinc-900 hover:bg-zinc-800 border border-zinc-700 rounded flex items-center gap-1 transition-colors cursor-pointer"
                title="Cycle to next workspace in alphabetical order (`clibm workspace switch` without ID/name)"
              >
                <span>Next (A→Z)</span>
                <ArrowRight className="w-3 h-3" />
              </button>
            </div>
            <span aria-hidden="true">·</span>
            <button
              type="button"
              onClick={() =>
                handleExecuteCommand(
                  `clibm config set auto_commit ${!repoState.config.auto_commit}`
                )
              }
              className={`px-2 py-0.5 rounded text-[11px] font-mono border flex items-center gap-1 transition-colors cursor-pointer ${
                repoState.config.auto_commit
                  ? 'bg-zinc-100 text-zinc-950 border-white font-bold'
                  : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:text-zinc-200'
              }`}
              title="Toggle automatic per-modification commits (`clibm config set auto_commit true|false`)"
            >
              <GitCommit className="w-3 h-3" />
              <span>AUTO-COMMIT: {repoState.config.auto_commit ? 'ON' : 'OFF'}</span>
            </button>
            <span aria-hidden="true">·</span>
            <span>
              Branch: <strong className="text-white">{activeBranch || 'detached'}</strong>
            </span>
            <span aria-hidden="true">·</span>
            <span className="tabular-nums">HEAD: {headCommit?.slice(0, 10)}</span>
            <span aria-hidden="true">·</span>
            <span className="tabular-nums">Tree: {repoState.stagedTreeHash.slice(0, 10)}</span>
            <span aria-hidden="true">·</span>
            <span className="tabular-nums">{objectCount} BLAKE3 CAS objects</span>
          </div>

          {/* Secondary Utility Controls */}
          <div className="flex items-center gap-2 flex-wrap">
            <button
              type="button"
              onClick={() => setActiveTab('config')}
              className="px-2.5 py-1 text-xs font-mono text-zinc-200 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-700 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title="Open Repository Configuration (.clibm/config.json)"
            >
              <Sliders className="w-3.5 h-3.5" />
              <span>Config</span>
            </button>
            <button
              type="button"
              onClick={downloadCoreOnlyZip}
              className="px-2.5 py-1 text-xs font-mono text-zinc-950 bg-zinc-200 hover:bg-white rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer font-medium"
              title="Download Core-Only Rust CLI without plugins (clibm-core-v0.1.0.zip)"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Core-Only (.zip)</span>
            </button>
            <button
              type="button"
              onClick={downloadRustCargoZip}
              className="px-2.5 py-1 text-xs font-mono text-zinc-200 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-700 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title="Download Full Workspace including TUI and Web plugins"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Full Workspace + Plugins (.zip)</span>
            </button>
            <button
              type="button"
              onClick={() => {
                setActiveTab('workbench');
                setWorkbenchSubTab('rust-source');
              }}
              className="px-2.5 py-1 text-xs font-mono text-zinc-300 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
            >
              <FolderGit2 className="w-3.5 h-3.5" />
              <span>Rust Source</span>
            </button>
            <button
              type="button"
              onClick={handleExportBundle}
              className="px-2.5 py-1 text-xs font-mono text-zinc-300 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title="Export .clibm CAS repository JSON bundle"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Export CAS Bundle</span>
            </button>
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              className="px-2.5 py-1 text-xs font-mono text-zinc-300 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title="Import .clibm CAS repository JSON bundle"
            >
              <Upload className="w-3.5 h-3.5" />
              <span>Import Bundle</span>
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".json"
              onChange={handleImportBundle}
              className="hidden"
            />
            <button
              type="button"
              onClick={() => handleExecuteCommand('clibm init')}
              className="px-2.5 py-1 text-xs font-mono text-zinc-400 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title="Reset repository to default seeded state"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>Reset Repo</span>
            </button>
          </div>
        </div>
      </div>

      {/* Main Content Container (1440px Desktop Baseline) */}
      <main className="flex-1 max-w-[1440px] w-full mx-auto px-6 py-6 space-y-6">
        {activeTab === 'showcase' && (
          <ShowcaseView
            repoState={repoState}
            activeBranch={activeBranch}
            terminalHistory={terminalHistory}
            onExecuteCommand={handleExecuteCommand}
            onClearTerminal={() => setTerminalHistory([])}
            onNavigateTab={(tab) => setActiveTab(tab)}
          />
        )}

        {activeTab === 'docs' && (
          <DocumentationView
            repoState={repoState}
            onUpdateRepoState={handleUpdateStateAndHistory}
          />
        )}

        {activeTab === 'plugins' && (
          <PluginMarketplaceView
            repoState={repoState}
            onRunCommand={(cmd) => {
              handleExecuteCommand(cmd);
              setActiveTab('workbench');
              setWorkbenchSubTab('terminal-kanban');
            }}
            onOpenTuiOrWeb={(mode) => {
              setTuiOrWebMode(mode);
              setActiveTab('tui-web');
            }}
          />
        )}

        {activeTab === 'tui-web' && (
          <TuiAndWebPluginsView
            repoState={repoState}
            onRunCommand={handleExecuteCommand}
            initialMode={tuiOrWebMode}
          />
        )}

        {activeTab === 'config' && (
          <div className="space-y-6">
            <ConfigurationView repoState={repoState} onRunCommand={handleExecuteCommand} />
            <TerminalPane
              activeBranch={activeBranch}
              stagedTreeHash={repoState.stagedTreeHash}
              history={terminalHistory}
              onExecute={handleExecuteCommand}
              onClear={() => setTerminalHistory([])}
              compact
            />
          </div>
        )}

        {activeTab === 'workbench' && (
          <div className="space-y-6">
            {/* Workbench Internal Sub-Navigation */}
            <div className="flex flex-wrap items-center justify-between gap-4 border-b border-zinc-800 pb-3">
              <div className="flex items-center gap-1.5 p-1 bg-zinc-900 border border-zinc-800 rounded-lg flex-wrap">
                <button
                  type="button"
                  onClick={() => setWorkbenchSubTab('terminal-kanban')}
                  className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                    workbenchSubTab === 'terminal-kanban'
                      ? 'bg-zinc-100 text-zinc-950 font-semibold'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  <Kanban className="w-3.5 h-3.5" />
                  <span>Terminal &amp; Kanban Board</span>
                </button>
                <button
                  type="button"
                  onClick={() => setWorkbenchSubTab('versioning')}
                  className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                    workbenchSubTab === 'versioning'
                      ? 'bg-zinc-100 text-zinc-950 font-semibold'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  <GitBranch className="w-3.5 h-3.5" />
                  <span>Version History &amp; Diff</span>
                </button>
                <button
                  type="button"
                  onClick={() => setWorkbenchSubTab('cas')}
                  className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                    workbenchSubTab === 'cas'
                      ? 'bg-zinc-100 text-zinc-950 font-semibold'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  <Database className="w-3.5 h-3.5" />
                  <span>CAS &amp; BLAKE3 Inspector</span>
                </button>
                <button
                  type="button"
                  onClick={() => setWorkbenchSubTab('config')}
                  className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                    workbenchSubTab === 'config'
                      ? 'bg-zinc-100 text-zinc-950 font-semibold'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  <Sliders className="w-3.5 h-3.5" />
                  <span>Config &amp; Auto-Commit</span>
                </button>
                <button
                  type="button"
                  onClick={() => setWorkbenchSubTab('rust-source')}
                  className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                    workbenchSubTab === 'rust-source'
                      ? 'bg-zinc-100 text-zinc-950 font-semibold'
                      : 'text-zinc-400 hover:text-white'
                  }`}
                >
                  <FolderGit2 className="w-3.5 h-3.5" />
                  <span>Rust Cargo Source &amp; Core Download</span>
                </button>
              </div>
            </div>

            {workbenchSubTab === 'terminal-kanban' && (
              <div className="space-y-6">
                <TerminalPane
                  activeBranch={activeBranch}
                  stagedTreeHash={repoState.stagedTreeHash}
                  history={terminalHistory}
                  onExecute={handleExecuteCommand}
                  onClear={() => setTerminalHistory([])}
                />
                <KanbanAndTasksView repoState={repoState} onRunCommand={handleExecuteCommand} />
              </div>
            )}

            {workbenchSubTab === 'versioning' && (
              <div className="space-y-6">
                <VersioningAndDiffView repoState={repoState} onRunCommand={handleExecuteCommand} />
                <TerminalPane
                  activeBranch={activeBranch}
                  stagedTreeHash={repoState.stagedTreeHash}
                  history={terminalHistory}
                  onExecute={handleExecuteCommand}
                  onClear={() => setTerminalHistory([])}
                  compact
                />
              </div>
            )}

            {workbenchSubTab === 'cas' && (
              <div className="space-y-6">
                <CasInspectorView repoState={repoState} onRunCommand={handleExecuteCommand} />
                <TerminalPane
                  activeBranch={activeBranch}
                  stagedTreeHash={repoState.stagedTreeHash}
                  history={terminalHistory}
                  onExecute={handleExecuteCommand}
                  onClear={() => setTerminalHistory([])}
                  compact
                />
              </div>
            )}

            {workbenchSubTab === 'config' && (
              <div className="space-y-6">
                <ConfigurationView repoState={repoState} onRunCommand={handleExecuteCommand} />
                <TerminalPane
                  activeBranch={activeBranch}
                  stagedTreeHash={repoState.stagedTreeHash}
                  history={terminalHistory}
                  onExecute={handleExecuteCommand}
                  onClear={() => setTerminalHistory([])}
                  compact
                />
              </div>
            )}

            {workbenchSubTab === 'rust-source' && <RustPackageView />}
          </div>
        )}
      </main>

      {/* Clean Quiet Monochromatic Footer */}
      <footer className="border-t border-zinc-800/80 bg-zinc-950 px-6 py-4 mt-auto">
        <div className="max-w-[1440px] mx-auto flex flex-wrap items-center justify-between gap-4 text-xs text-zinc-500">
          <div className="flex items-center gap-2 font-mono">
            <span>CLIBM v0.1.0</span>
            <span aria-hidden="true">·</span>
            <span>Rust 2021 + BLAKE3 CAS + Monochromatic System</span>
          </div>
          <div className="flex flex-wrap items-center gap-5">
            <button
              type="button"
              onClick={() => setActiveTab('showcase')}
              className="hover:text-zinc-200 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <Sparkles className="w-3.5 h-3.5" />
              <span>Showcase</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('docs')}
              className="hover:text-zinc-200 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <BookOpen className="w-3.5 h-3.5" />
              <span>Documentation</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('plugins')}
              className="hover:text-zinc-200 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <Package className="w-3.5 h-3.5" />
              <span>Plugins</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('tui-web')}
              className="hover:text-zinc-200 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <Cpu className="w-3.5 h-3.5" />
              <span>TUI &amp; Web App</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('workbench')}
              className="hover:text-zinc-200 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <Terminal className="w-3.5 h-3.5" />
              <span>CLI Workbench</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('config')}
              className="hover:text-zinc-200 transition-colors flex items-center gap-1 cursor-pointer"
            >
              <Sliders className="w-3.5 h-3.5" />
              <span>Config</span>
            </button>
            <button
              type="button"
              onClick={downloadCoreOnlyZip}
              className="text-zinc-200 hover:text-white transition-colors flex items-center gap-1 font-medium cursor-pointer"
            >
              <Download className="w-3.5 h-3.5" />
              <span>Download Core CLI (.zip)</span>
            </button>
          </div>
        </div>
      </footer>
    </div>
  );
}
