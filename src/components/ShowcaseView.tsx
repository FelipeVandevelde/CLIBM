import React, { useState, useMemo } from 'react';
import {
  Download,
  BookOpen,
  Package,
  Terminal,
  Globe,
  Database,
  ShieldCheck,
  ArrowRight,
  Cpu,
} from 'lucide-react';
import { ClibmRepositoryState, TerminalHistoryEntry } from '../core/types';
import { computeCanonicalObjectHash } from '../core/blake3';
import {
  downloadCoreOnlyZip,
  downloadPluginZip,
  downloadRustCargoZip,
} from '../rust/rustSourceRegistry';
import { TerminalPane } from './TerminalPane';
import { TuiAndWebPluginsView } from './TuiAndWebPluginsView';
import { CasInspectorView } from './CasInspectorView';

interface ShowcaseViewProps {
  repoState: ClibmRepositoryState;
  activeBranch: string | null;
  terminalHistory: TerminalHistoryEntry[];
  onExecuteCommand: (cmd: string) => void;
  onClearTerminal: () => void;
  onNavigateTab: (tab: 'docs' | 'plugins' | 'tui-web' | 'workbench') => void;
}

export const ShowcaseView: React.FC<ShowcaseViewProps> = ({
  repoState,
  activeBranch,
  terminalHistory,
  onExecuteCommand,
  onClearTerminal,
  onNavigateTab,
}) => {
  const [demoStageMode, setDemoStageMode] = useState<'cli' | 'tui' | 'web' | 'cas'>('cli');

  // Interactive Live Canonical JSON -> BLAKE3 Hash Tester in Hero
  const [sampleTitle, setSampleTitle] = useState(
    'Implement BLAKE3 Content-Addressable Storage in Rust'
  );
  const [samplePriority, setSamplePriority] = useState<'critical' | 'high' | 'medium' | 'low'>(
    'critical'
  );
  const [sampleColumn, setSampleColumn] = useState<'todo' | 'in-progress' | 'done'>('in-progress');

  const liveHashDemo = useMemo(() => {
    const payload = {
      id: 'CLIBM-DEMO',
      title: sampleTitle,
      priority: samplePriority,
      column_id: sampleColumn,
      tags: ['rust', 'blake3', 'cas'],
    };
    return computeCanonicalObjectHash('task', '0.1.0', payload);
  }, [sampleTitle, samplePriority, sampleColumn]);

  const objectCount = Object.keys(repoState.objects).length;

  return (
    <div className="space-y-12">
      {/* 1. HERO SECTION: Proposition + Core-Only Download + Live Canonical BLAKE3 Verifier */}
      <section className="border border-zinc-800 bg-zinc-900/50 rounded-xl p-6 md:p-10">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-stretch">
          {/* Left 7 Cols: Headline, Value Proposition & Primary Actions */}
          <div className="lg:col-span-7 flex flex-col justify-between space-y-6">
            <div className="space-y-4">
              <div className="flex flex-wrap items-center gap-2 text-xs font-mono text-zinc-300">
                <span className="text-white font-semibold">CLIBM v0.1.0</span>
                <span aria-hidden="true">·</span>
                <span>Rust 2021 Edition</span>
                <span aria-hidden="true">·</span>
                <span>Linux &amp; Windows Native</span>
                <span aria-hidden="true">·</span>
                <span>Zero External Databases</span>
              </div>

              <h1
                className="font-bold tracking-tight text-white leading-[1.12]"
                style={{
                  fontSize: 'clamp(2.25rem, 3.6vw, 3.25rem)',
                  textWrap: 'balance',
                }}
              >
                Local-First CLI Project Management Built on BLAKE3 Content-Addressable Storage.
              </h1>

              <p className="text-base text-zinc-300 leading-relaxed max-w-[64ch]">
                CLIBM replaces heavyweight cloud trackers and relational databases with a fast,
                portable Rust core engine. Organize workspaces, tasks, dependencies, and Kanban
                workflows as immutable, cryptographically verified JSON objects with Git-style
                commits, branches, structural diffs, and optional TUI &amp; Web plugins.
              </p>
            </div>

            {/* Action Bar: Core-Only Download + Full Workspace + Docs */}
            <div className="space-y-4 pt-2">
              <div className="flex flex-wrap items-center gap-3">
                <button
                  type="button"
                  onClick={downloadCoreOnlyZip}
                  className="px-5 py-3 text-sm font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded-lg flex items-center gap-2 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                >
                  <Download className="w-4 h-4" />
                  <span>Download Core CLI Only (clibm-core-v0.1.0.zip)</span>
                </button>

                <button
                  type="button"
                  onClick={downloadRustCargoZip}
                  className="px-4 py-3 text-sm font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-100 border border-zinc-700 rounded-lg flex items-center gap-2 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                >
                  <Download className="w-4 h-4 text-zinc-300" />
                  <span>Full Workspace + Plugins (.zip)</span>
                </button>

                <button
                  type="button"
                  onClick={() => onNavigateTab('docs')}
                  className="px-4 py-3 text-sm font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-200 border border-zinc-800 rounded-lg flex items-center gap-2 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                >
                  <BookOpen className="w-4 h-4 text-zinc-300" />
                  <span>Documentation</span>
                </button>

                <button
                  type="button"
                  onClick={() => onNavigateTab('plugins')}
                  className="px-4 py-3 text-sm font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-200 border border-zinc-800 rounded-lg flex items-center gap-2 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                >
                  <Package className="w-4 h-4 text-zinc-300" />
                  <span>Plugin Marketplace</span>
                </button>
              </div>

              <div className="flex flex-wrap items-center gap-3 text-xs font-mono text-zinc-400">
                <span>Core Package (0 UI Crates):</span>
                <span className="text-zinc-100">clibm (CLI)</span>
                <span aria-hidden="true">·</span>
                <span className="text-zinc-100">clibm_core (CAS + DAG Engine)</span>
                <span aria-hidden="true">|</span>
                <span>Optional Plugins:</span>
                <span className="text-zinc-300">clibm-tui (Ratatui)</span>
                <span aria-hidden="true">·</span>
                <span className="text-zinc-300">clibm-web (Local HTTP Server)</span>
              </div>
            </div>
          </div>

          {/* Right 5 Cols: Interactive Canonical JSON -> BLAKE3 Hash Verifier */}
          <div className="lg:col-span-5 border border-zinc-800 bg-zinc-950 rounded-lg p-5 flex flex-col justify-between space-y-4">
            <div className="space-y-1 border-b border-zinc-800 pb-3">
              <div className="flex items-center justify-between text-xs font-mono">
                <span className="text-zinc-200 font-semibold">
                  LIVE ENGINE PROOF · SPEC 2.2.2
                </span>
                <span className="text-zinc-100 flex items-center gap-1">
                  <ShieldCheck className="w-3.5 h-3.5" />
                  RFC 8785 + BLAKE3
                </span>
              </div>
              <h2 className="text-sm font-semibold text-white">
                Interactive Content-Addressable Object Hasher
              </h2>
              <p className="text-xs text-zinc-400">
                Modify any field below to observe how CLIBM normalizes keys into canonical JSON and
                computes the immutable 256-bit BLAKE3 identifier in real time.
              </p>
            </div>

            <div className="space-y-3">
              <div>
                <label className="block text-[11px] font-mono text-zinc-400 mb-1">
                  Payload.title
                </label>
                <input
                  type="text"
                  value={sampleTitle}
                  onChange={(e) => setSampleTitle(e.target.value)}
                  className="w-full px-3 py-1.5 text-xs font-mono bg-zinc-900 border border-zinc-800 rounded text-white focus:outline-none focus:border-zinc-400"
                />
              </div>

              <div className="grid grid-cols-2 gap-2.5">
                <div>
                  <label className="block text-[11px] font-mono text-zinc-400 mb-1">
                    Payload.priority
                  </label>
                  <select
                    value={samplePriority}
                    onChange={(e) =>
                      setSamplePriority(e.target.value as 'critical' | 'high' | 'medium' | 'low')
                    }
                    className="w-full px-2.5 py-1.5 text-xs font-mono bg-zinc-900 border border-zinc-800 rounded text-white"
                  >
                    <option value="critical">critical</option>
                    <option value="high">high</option>
                    <option value="medium">medium</option>
                    <option value="low">low</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[11px] font-mono text-zinc-400 mb-1">
                    Payload.column_id
                  </label>
                  <select
                    value={sampleColumn}
                    onChange={(e) =>
                      setSampleColumn(e.target.value as 'todo' | 'in-progress' | 'done')
                    }
                    className="w-full px-2.5 py-1.5 text-xs font-mono bg-zinc-900 border border-zinc-800 rounded text-white"
                  >
                    <option value="todo">todo</option>
                    <option value="in-progress">in-progress</option>
                    <option value="done">done</option>
                  </select>
                </div>
              </div>

              <div className="p-3 bg-zinc-900/90 border border-zinc-800 rounded font-mono text-[11px] space-y-1.5">
                <div className="text-zinc-400">
                  Storage Path:{' '}
                  <span className="text-zinc-200">
                    .clibm/objects/{liveHashDemo.hash.slice(0, 2)}/{liveHashDemo.hash.slice(2, 24)}
                    ...json
                  </span>
                </div>
                <div className="text-zinc-400">
                  BLAKE3 Digest (256-bit):
                  <div className="text-white break-all font-semibold mt-0.5 tabular-nums">
                    {liveHashDemo.hash}
                  </div>
                </div>
              </div>
            </div>

            <button
              type="button"
              onClick={() =>
                onExecuteCommand(
                  `clibm task add "${sampleTitle.replace(/"/g, "'")}" -p ${samplePriority} -c ${sampleColumn} --tags showcase`
                )
              }
              className="w-full py-2 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded transition-colors flex items-center justify-center gap-1.5 cursor-pointer"
            >
              <span>Commit This Task Object to Live Repository</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </section>

      {/* 2. INTERACTIVE 4-MODE LIVE DEMONSTRATION STAGE */}
      <section className="space-y-4">
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4">
          <div className="space-y-1">
            <div className="text-xs font-mono text-zinc-400">
              INTERACTIVE LIVE SANDBOX · 1 CORE ENGINE, 4 INTERFACES
            </div>
            <h2 className="text-xl font-bold text-white tracking-tight">
              Experience the CLI, Ratatui TUI, and Self-Hosted Web Plugins Live
            </h2>
            <p className="text-xs text-zinc-400">
              All four views below read and mutate the exact same local BLAKE3 repository (
              <span className="font-mono text-zinc-200 tabular-nums">{objectCount} objects</span>{' '}
              on branch <span className="font-mono text-white font-semibold">{activeBranch}</span>).
            </p>
          </div>

          {/* 4-Mode Switcher */}
          <div className="flex flex-wrap items-center gap-1.5 p-1 bg-zinc-900 border border-zinc-800 rounded-lg">
            <button
              type="button"
              onClick={() => setDemoStageMode('cli')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 ${
                demoStageMode === 'cli'
                  ? 'bg-zinc-100 text-zinc-950 font-semibold'
                  : 'text-zinc-400 hover:text-white'
              }`}
            >
              <Terminal className="w-3.5 h-3.5" />
              <span>01. CLI Terminal (clibm)</span>
            </button>
            <button
              type="button"
              onClick={() => setDemoStageMode('tui')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 ${
                demoStageMode === 'tui'
                  ? 'bg-zinc-100 text-zinc-950 font-semibold'
                  : 'text-zinc-400 hover:text-white'
              }`}
            >
              <Cpu className="w-3.5 h-3.5" />
              <span>02. Ratatui TUI (clibm-tui)</span>
            </button>
            <button
              type="button"
              onClick={() => setDemoStageMode('web')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 ${
                demoStageMode === 'web'
                  ? 'bg-zinc-100 text-zinc-950 font-semibold'
                  : 'text-zinc-400 hover:text-white'
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>03. Self-Hosted Web (clibm-web)</span>
            </button>
            <button
              type="button"
              onClick={() => setDemoStageMode('cas')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 ${
                demoStageMode === 'cas'
                  ? 'bg-zinc-100 text-zinc-950 font-semibold'
                  : 'text-zinc-400 hover:text-white'
              }`}
            >
              <Database className="w-3.5 h-3.5" />
              <span>04. BLAKE3 CAS Store</span>
            </button>
          </div>
        </div>

        {/* Demo Stage Viewport */}
        <div>
          {demoStageMode === 'cli' && (
            <TerminalPane
              activeBranch={activeBranch}
              stagedTreeHash={repoState.stagedTreeHash}
              history={terminalHistory}
              onExecute={onExecuteCommand}
              onClear={onClearTerminal}
            />
          )}
          {demoStageMode === 'tui' && (
            <TuiAndWebPluginsView
              repoState={repoState}
              onRunCommand={onExecuteCommand}
              initialMode="tui"
            />
          )}
          {demoStageMode === 'web' && (
            <TuiAndWebPluginsView
              repoState={repoState}
              onRunCommand={onExecuteCommand}
              initialMode="web"
            />
          )}
          {demoStageMode === 'cas' && (
            <CasInspectorView repoState={repoState} onRunCommand={onExecuteCommand} />
          )}
        </div>
      </section>

      {/* 3. ASYMMETRIC CAPABILITY BENTO GRID & QUANTITATIVE PROOF */}
      <section className="space-y-4">
        <div className="space-y-1">
          <div className="text-xs font-mono text-zinc-400">
            ARCHITECTURAL MECHANISMS &amp; QUANTITATIVE SPECS
          </div>
          <h2 className="text-xl font-bold text-white tracking-tight">
            Engineered for Deterministic Speed, Portability, and Extensibility
          </h2>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
          {/* Card 01 (col-span-2) */}
          <div className="lg:col-span-2 border border-zinc-800 bg-zinc-900/50 rounded-lg p-6 flex flex-col justify-between space-y-4">
            <div className="space-y-2">
              <div className="text-xs font-mono text-zinc-300">
                01. Content-Addressable Storage &amp; Canonical JSON
              </div>
              <h3 className="text-base font-semibold text-white">
                Zero SQLite or Relational Overhead — Pure Immutable BLAKE3 Object Graph
              </h3>
              <p className="text-sm text-zinc-300 leading-relaxed">
                Every workspace, project, Kanban board, task, tree snapshot, and commit is stored
                as an immutable file identified by the 256-bit BLAKE3 hash of its RFC 8785
                Canonical JSON representation. Unchanged tasks are shared across hundreds of
                commits with zero data duplication.
              </p>
            </div>
            <div className="pt-3 border-t border-zinc-800 flex flex-wrap items-center justify-between gap-3 text-xs font-mono text-zinc-400">
              <span>Digest: 64-char hex (256-bit BLAKE3)</span>
              <span>·</span>
              <span>Sharding: 256 fan-out directories (.clibm/objects/00..ff)</span>
              <span>·</span>
              <button
                type="button"
                onClick={() => setDemoStageMode('cas')}
                className="text-white hover:underline"
              >
                Inspect CAS Store →
              </button>
            </div>
          </div>

          {/* Card 02 (col-span-1) */}
          <div className="border border-zinc-800 bg-zinc-900/50 rounded-lg p-6 flex flex-col justify-between space-y-4">
            <div className="space-y-2">
              <div className="text-xs font-mono text-zinc-300">
                02. Git-Style Versioning &amp; Structural Diffs
              </div>
              <h3 className="text-base font-semibold text-white">
                Branch, Diff, Checkout &amp; Merge Project States
              </h3>
              <p className="text-sm text-zinc-300 leading-relaxed">
                Create experimental workflow branches (<code className="text-zinc-200">clibm branch</code>),
                compare any two versions at the field level (<code className="text-zinc-200">clibm diff</code>),
                and recover historical project snapshots in <span className="tabular-nums">&lt;1ms</span>.
              </p>
            </div>
            <div className="pt-3 border-t border-zinc-800 text-xs font-mono text-zinc-400 flex items-center justify-between">
              <span>O(1) subtree equality check</span>
              <button
                type="button"
                onClick={() => onNavigateTab('docs')}
                className="text-white hover:underline"
              >
                Versioning Docs →
              </button>
            </div>
          </div>

          {/* Card 03 (col-span-1) */}
          <div className="border border-zinc-800 bg-zinc-900/50 rounded-lg p-6 flex flex-col justify-between space-y-4">
            <div className="space-y-2">
              <div className="text-xs font-mono text-zinc-300">
                03. Standalone Core CLI Download
              </div>
              <h3 className="text-base font-semibold text-white">
                Lightweight Core Binary Without Plugin Overhead
              </h3>
              <p className="text-sm text-zinc-300 leading-relaxed">
                Need strictly the command-line tool and headless engine? Download{' '}
                <code className="text-zinc-200">clibm-core-v0.1.0.zip</code> containing only{' '}
                <code className="text-zinc-200">clibm_core</code> and <code className="text-zinc-200">clibm</code>{' '}
                with zero TUI or HTTP dependencies.
              </p>
            </div>
            <div className="pt-3 border-t border-zinc-800 flex items-center justify-between">
              <span className="text-xs font-mono text-zinc-400">14 core files · 0 UI crates</span>
              <button
                type="button"
                onClick={downloadCoreOnlyZip}
                className="px-2.5 py-1 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded cursor-pointer"
              >
                Download Core (.zip)
              </button>
            </div>
          </div>

          {/* Card 04 (col-span-2) */}
          <div className="lg:col-span-2 border border-zinc-800 bg-zinc-900/50 rounded-lg p-6 flex flex-col justify-between space-y-4">
            <div className="space-y-2">
              <div className="text-xs font-mono text-zinc-300">
                04. Optional TUI &amp; Self-Hosted Web Server Plugins
              </div>
              <h3 className="text-base font-semibold text-white">
                Extend When Needed via Ratatui Terminal UI or Localhost HTTP Server
              </h3>
              <p className="text-sm text-zinc-300 leading-relaxed">
                Download <code className="text-zinc-100">clibm-tui</code> for a full-screen Ratatui + Crossterm
                terminal dashboard, or run <code className="text-zinc-100">clibm-web</code> to spin up a
                single-binary local HTTP server on <span className="font-mono">127.0.0.1:7878</span> with an
                embedded web application.
              </p>
            </div>
            <div className="pt-3 border-t border-zinc-800 flex flex-wrap items-center gap-3">
              <button
                type="button"
                onClick={() => downloadPluginZip('clibm-tui')}
                className="px-3 py-1.5 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-100 rounded flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Download clibm-tui.zip</span>
              </button>
              <button
                type="button"
                onClick={() => downloadPluginZip('clibm-web')}
                className="px-3 py-1.5 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-100 rounded flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Download clibm-web.zip</span>
              </button>
              <button
                type="button"
                onClick={() => onNavigateTab('plugins')}
                className="ml-auto text-xs font-mono text-white hover:underline"
              >
                View All 5 Plugins →
              </button>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
};
