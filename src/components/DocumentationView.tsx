import React, { useState, useMemo } from 'react';
import {
  BookOpen,
  Search,
  Play,
  Download,
  Check,
  Copy,
  Terminal,
} from 'lucide-react';
import { ClibmRepositoryState } from '../core/types';
import { executeCliCommand } from '../core/clibmCore';
import {
  downloadCoreOnlyZip,
  downloadPluginZip,
  downloadRustCargoZip,
} from '../rust/rustSourceRegistry';

interface DocumentationViewProps {
  repoState: ClibmRepositoryState;
  onUpdateRepoState: (
    next: ClibmRepositoryState,
    cmd: string,
    output: string,
    status: 'ok' | 'error' | 'info'
  ) => void;
}

interface DocSnippet {
  id: string;
  label: string;
  command: string;
  explanation: string;
}

interface DocChapter {
  id: string;
  number: string;
  title: string;
  summary: string;
  sections: Array<{
    heading: string;
    body: string[];
    codeBlock?: string;
    runnableSnippets?: DocSnippet[];
  }>;
}

const DOC_CHAPTERS: DocChapter[] = [
  {
    id: 'getting-started',
    number: '01',
    title: 'Getting Started & Native Installation',
    summary:
      'Compile and run the standalone Core CLI (`clibm-core-v0.1.0.zip`) or the Full Cargo Workspace on Linux and Windows.',
    sections: [
      {
        heading: 'Option A — Core CLI Only (`clibm-core-v0.1.0.zip`)',
        body: [
          'For users who want strictly the headless `clibm_core` engine and standard `clibm` command-line binary without any TUI or HTTP server dependencies, download `clibm-core-v0.1.0.zip`. It compiles in seconds using only `serde`, `serde_json`, `blake3`, `clap`, `chrono`, and `anyhow`.',
        ],
        codeBlock: `# Build the standalone Core CLI binary (zero external UI plugins)
unzip clibm-core-v0.1.0.zip
cd clibm-core
cargo build --release

# Resulting native binary (Linux & Windows):
# ./target/release/clibm       (or .\\target\\release\\clibm.exe)`,
      },
      {
        heading: 'Option B — Full Workspace + TUI & Web Plugins (`clibm-v0.1.0-workspace.zip`)',
        body: [
          'To build the core CLI together with the official `clibm-tui` (Ratatui + Crossterm) and `clibm-web` (localhost HTTP server) plugins, download the Full Workspace archive.',
        ],
        codeBlock: `unzip clibm-v0.1.0-workspace.zip
cd clibm
cargo build --release --workspace`,
      },
      {
        heading: 'Interactive Quick-Start Commands',
        body: [
          'Every command below is wired directly to the live BLAKE3 Content-Addressable Storage engine in this browser session. Click "Run Live" on any command to execute it and inspect the output immediately.',
        ],
        runnableSnippets: [
          {
            id: 'qs-status',
            label: 'Inspect repository status & active BLAKE3 tree hash',
            command: 'clibm status',
            explanation:
              'Resolves .clibm/HEAD, compares the HEAD commit tree against STAGED_TREE, and lists any uncommitted modifications.',
          },
          {
            id: 'qs-board',
            label: 'Render the active Kanban board in ASCII',
            command: 'clibm board show',
            explanation:
              'Reads the active KanbanBoardPayload and groups all tasks in the current working tree by column order and WIP limit.',
          },
          {
            id: 'qs-fsck',
            label: 'Verify BLAKE3 cryptographic integrity of all objects',
            command: 'clibm fsck',
            explanation:
              'Re-normalizes every stored JSON object into RFC 8785 Canonical JSON and verifies its 256-bit BLAKE3 hash.',
          },
        ],
      },
    ],
  },
  {
    id: 'architectural-principles',
    number: '02',
    title: 'Architectural Principles & Decoupled Core',
    summary:
      'How CLIBM enforces strict separation between its headless business logic, immutable storage, and user interfaces.',
    sections: [
      {
        heading: 'The Seven Governing Principles (Spec 1.1)',
        body: [
          '1. Interface Independence: `clibm_core` never imports terminal or DOM libraries. The CLI (`clap`), TUI (`ratatui`), and Self-Hosted Web Server (`tiny_http`) all call the same headless `ClibmEngine` API.',
          '2. Command & API Contracts: Every mutation is expressed as an atomic command that produces a new immutable object hash and updates the staged tree snapshot.',
          '3. Immutable Hash-Addressed Objects: Entities are never mutated in place. Editing a task writes a new task object with a new BLAKE3 digest while preserving the prior object for historical commits.',
          '4. Permanent History Preservation: Deleting or moving a task only updates the current `TreeSnapshotPayload` pointer map; historical commits remain 100% recoverable.',
          '5. Defined Extension Contracts: Plugins communicate via `CLIBM_PLUGIN_CONTRACT_V1` (structured JSON over stdin/stdout or direct crate linkage).',
          '6. Independent Component Modularity: `canonical.rs`, `cas.rs`, `index.rs`, `versioning.rs`, and `plugin.rs` operate as self-contained modules.',
          '7. Minimal Dependency Footprint: The Core-Only package avoids all unnecessary dependencies and compiles to a single portable native binary.',
        ],
      },
    ],
  },
  {
    id: 'cas-and-blake3',
    number: '03',
    title: 'BLAKE3 Content-Addressable Storage & Object Spec',
    summary:
      'Specification 2.2: Canonical JSON serialization, 256-bit BLAKE3 identifiers, sharded object storage, and computed indices.',
    sections: [
      {
        heading: 'Canonical Object Envelope (Spec 2.2.2)',
        body: [
          'Traditional relational databases (SQLite, PostgreSQL, MySQL) are intentionally excluded. Instead, CLIBM stores every entity (`workspace`, `project`, `board`, `task`, `tree`, `commit`) as an immutable file under `.clibm/objects/<2-hex-shard>/<62-hex-rest>.json`.',
          'Before computing the BLAKE3 hash, `canonical::compute_object_hash` recursively sorts all JSON map keys lexicographically (RFC 8785 Canonical JSON) across `Type`, `Version`, and `Payload`. This guarantees identical hashes across Linux and Windows regardless of field insertion order.',
        ],
        codeBlock: `{
  "Type": "task",
  "Version": "0.1.0",
  "Hash": "7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
  "Payload": {
    "assignee": "V. Souza",
    "board_id": "BRD-01",
    "column_id": "done",
    "comments": [],
    "created_at": "2026-11-02T09:00:00.000Z",
    "depends_on": [],
    "description": "Ensure deterministic key ordering on all serialized structs...",
    "due_date": "2026-11-05",
    "id": "CLIBM-101",
    "metadata": { "module": "src/cas.rs" },
    "priority": "critical",
    "project_id": "PRJ-01",
    "tags": ["cas", "blake3", "rust"],
    "title": "Implement RFC 8785 Canonical JSON & BLAKE3 CAS envelope",
    "updated_at": "2026-11-03T14:20:00.000Z"
  }
}`,
        runnableSnippets: [
          {
            id: 'cas-cat',
            label: 'Inspect repository status and object count',
            command: 'clibm status',
            explanation:
              'Use `clibm cat-object <hash-prefix>` with any 4+ character BLAKE3 prefix to inspect the exact canonical JSON envelope.',
          },
        ],
      },
    ],
  },
  {
    id: 'cli-reference',
    number: '04',
    title: 'Complete CLI Command Reference',
    summary:
      'Syntax, flags, and live executable examples for workspace, project, task, and Kanban board operations.',
    sections: [
      {
        heading: 'Task & Dependency Commands',
        body: [
          'Tasks support unique IDs (`CLIBM-101`), priorities (`low`, `medium`, `high`, `critical`), deadlines, tags, comments, blocking dependencies (`depends_on`), and individual change histories across commits.',
        ],
        runnableSnippets: [
          {
            id: 'cli-task-list',
            label: 'List all tasks in the current working tree',
            command: 'clibm task list',
            explanation:
              'Outputs tabular task ID, priority, column, short BLAKE3 hash, dependencies, and title.',
          },
          {
            id: 'cli-task-add',
            label: 'Create a new high-priority task with tags and dependency',
            command:
              'clibm task add "Write Windows PowerShell release packaging test" -p high -c todo -a "V. Souza" --tags windows,ci --depends-on CLIBM-101',
            explanation:
              'Serializes the new TaskPayload, stores it in .clibm/objects, updates the staged TreeSnapshotPayload, and rebuilds computed indices.',
          },
          {
            id: 'cli-task-show',
            label: 'Inspect full task payload and comment thread',
            command: 'clibm task show CLIBM-101',
            explanation: 'Displays all fields, tags, dependencies, and timestamped comments for CLIBM-101.',
          },
          {
            id: 'cli-task-hist',
            label: 'Trace individual change history of CLIBM-101 across commits',
            command: 'clibm task history CLIBM-101',
            explanation:
              'Walks the commit DAG and extracts every distinct immutable task object hash associated with CLIBM-101.',
          },
        ],
      },
      {
        heading: 'Kanban Board & Workflow Commands',
        body: [
          'Kanban boards define ordered columns, optional WIP limits, and workflow enforcement rules (`enforce_dependencies`). Attempting to move a task into an active column before its blocking dependencies are in `done` is automatically rejected unless `--force` is passed.',
        ],
        runnableSnippets: [
          {
            id: 'cli-board-move',
            label: 'Move CLIBM-103 from in-progress to review',
            command: 'clibm board move CLIBM-103 review',
            explanation:
              'Verifies that all tasks in CLIBM-103 depends_on (CLIBM-101, CLIBM-102) are in a completed state before updating column_id.',
          },
        ],
      },
    ],
  },
  {
    id: 'versioning-and-diffs',
    number: '05',
    title: 'Git-Style Versioning, Branching, Diffs & Merges',
    summary:
      'How CLIBM snapshots project state, branches without data duplication, computes structural diffs, and merges branches.',
    sections: [
      {
        heading: 'Tree Snapshots, Commits, Diffs & Merges (Spec 2.1.4)',
        body: [
          'Instead of copying the entire database on every version, a commit points to a compact `TreeSnapshotPayload` that maps entity IDs (`CLIBM-101`) to their current 64-char BLAKE3 object hash. Unchanged tasks share the exact same file on disk across all commits and branches.',
        ],
        runnableSnippets: [
          {
            id: 'ver-log',
            label: 'Traverse the commit history DAG',
            command: 'clibm log',
            explanation:
              'Lists commits in reverse chronological order with commit hash, tree hash, author, and message.',
          },
          {
            id: 'ver-diff',
            label: 'Run structural diff between main and feat/plugin-hooks',
            command: 'clibm diff main feat/plugin-hooks',
            explanation:
              'Compares both TreeSnapshotPayloads in O(N) time by comparing BLAKE3 hashes and detailing exact field changes.',
          },
          {
            id: 'ver-merge',
            label: 'Merge feat/plugin-hooks branch into current HEAD',
            command: 'clibm merge feat/plugin-hooks',
            explanation:
              'Combines the tree mappings from feat/plugin-hooks into the active branch and records a two-parent merge commit.',
          },
        ],
      },
    ],
  },
  {
    id: 'plugin-contract',
    number: '06',
    title: 'Plugin Contract Specification (CLIBM_PLUGIN_CONTRACT_V1)',
    summary:
      'Defined communication protocol for extending CLIBM with custom commands, lifecycle hooks, and external binaries.',
    sections: [
      {
        heading: 'JSON Stdin/Stdout Protocol & Hook Lifecycle',
        body: [
          'External plugins on Linux and Windows receive a structured `PluginInvocationRequest` JSON payload over `stdin` containing the contract version (`CLIBM_PLUGIN_CONTRACT_V1`), lifecycle event (`pre-commit`, `pre-move`, `on-task-depend`, `manual-run`), current `tree_hash`, and CLI arguments, and return a `PluginInvocationResponse` over `stdout`.',
        ],
        codeBlock: `// Request sent over stdin to plugin executable:
{
  "contract": "CLIBM_PLUGIN_CONTRACT_V1",
  "event": "pre-commit",
  "tree_hash": "9a4f82e01c7b...",
  "args": ["--strict"]
}

// Response returned over stdout by plugin executable:
{
  "ok": true,
  "output": "[dep-audit] Audited 6 tasks: DAG is acyclic (0 circular dependencies)."
}`,
        runnableSnippets: [
          {
            id: 'plg-audit',
            label: 'Execute dep-audit plugin via CLIBM_PLUGIN_CONTRACT_V1',
            command: 'clibm plugin run dep-audit',
            explanation:
              'Audits the active working tree for circular dependencies and critical-path blockers.',
          },
          {
            id: 'plg-md',
            label: 'Execute export-md plugin to generate Markdown table report',
            command: 'clibm plugin run export-md',
            explanation:
              'Renders the current tree snapshot into a portable Markdown sprint report.',
          },
        ],
      },
    ],
  },
  {
    id: 'tui-and-web-guides',
    number: '07',
    title: 'Ratatui TUI & Self-Hosted Web Server Plugins',
    summary:
      'Folder-scoped installation and ongoing-project usage guides for the standalone clibm-tui and clibm-web crates.',
    sections: [
      {
        heading: 'Installing & Running clibm-tui (Terminal User Interface)',
        body: [
          'The `clibm-tui` crate is a standalone Cargo package located in its own folder (`plugins/clibm-tui`) with its own `README.md`, `install.sh`, and `install.ps1`. It must be compiled and installed from within its own folder.',
          'Once installed, `clibm-tui` is placed on your user `PATH` (`~/.local/bin/clibm-tui`) and auto-registered in `~/.clibm/plugins/registry.json`. Navigate to any ongoing CLIBM project directory (containing `.clibm/`) and run `clibm plugin run tui` (or `clibm-tui`) to open your live project board.',
        ],
        codeBlock: `# 1. Install strictly from within the clibm-tui folder:
cd plugins/clibm-tui
./install.sh          # (or on Windows: .\\install.ps1, or: clibm plugin install .)

# 2. Run inside any ongoing CLIBM project folder:
cd ~/my-ongoing-project
clibm plugin run tui  # or run binary directly: clibm-tui`,
        runnableSnippets: [
          {
            id: 'tui-run-sim',
            label: 'Simulate running clibm-tui inside the active ongoing project',
            command: 'clibm plugin run tui',
            explanation:
              'Attaches to the ongoing .clibm/ repository in the current or parent directory and launches the Ratatui screen.',
          },
        ],
      },
      {
        heading: 'Installing & Running clibm-web (Single-Binary Self-Hosted Web Server)',
        body: [
          'The `clibm-web` crate is a standalone Cargo package located in `plugins/clibm-web` with its own `README.md` and install scripts (`ui.html` is embedded at compile time via `include_str!("ui.html")`).',
          'After installing `clibm-web` from its folder, navigate to any ongoing project directory and run `clibm plugin run web -- --port 7878` or `clibm-web --port 7878` to serve your project on `http://127.0.0.1:7878`.',
        ],
        codeBlock: `# 1. Install strictly from within the clibm-web folder:
cd plugins/clibm-web
./install.sh          # (or on Windows: .\\install.ps1, or: clibm plugin install .)

# 2. Run inside any ongoing CLIBM project folder:
cd ~/my-ongoing-project
clibm plugin run web -- --port 7878  # or run binary directly: clibm-web --port 7878
# Open http://127.0.0.1:7878 in your browser`,
        runnableSnippets: [
          {
            id: 'web-run-sim',
            label: 'Simulate launching clibm-web inside the active ongoing project',
            command: 'clibm plugin run web -- --port 7878',
            explanation:
              'Discovers the ongoing .clibm/ repository and binds the embedded HTTP workbench to 127.0.0.1:7878.',
          },
        ],
      },
    ],
  },
];

export const DocumentationView: React.FC<DocumentationViewProps> = ({
  repoState,
  onUpdateRepoState,
}) => {
  const [selectedChapterId, setSelectedChapterId] = useState<string>(DOC_CHAPTERS[0].id);
  const [searchQuery, setSearchQuery] = useState('');
  const [snippetOutputs, setSnippetOutputs] = useState<
    Record<string, { output: string; status: string }>
  >({});
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const filteredChapters = useMemo(() => {
    if (!searchQuery.trim()) return DOC_CHAPTERS;
    const q = searchQuery.toLowerCase();
    return DOC_CHAPTERS.filter(
      (c) =>
        c.title.toLowerCase().includes(q) ||
        c.summary.toLowerCase().includes(q) ||
        c.sections.some(
          (s) =>
            s.heading.toLowerCase().includes(q) ||
            s.body.some((b) => b.toLowerCase().includes(q)) ||
            (s.codeBlock && s.codeBlock.toLowerCase().includes(q))
        )
    );
  }, [searchQuery]);

  const activeChapter =
    filteredChapters.find((c) => c.id === selectedChapterId) ||
    filteredChapters[0] ||
    DOC_CHAPTERS[0];

  const handleRunSnippet = (snippet: DocSnippet) => {
    const res = executeCliCommand(repoState, snippet.command);
    onUpdateRepoState(res.nextState, snippet.command, res.output, res.status);
    setSnippetOutputs((prev) => ({
      ...prev,
      [snippet.id]: { output: res.output, status: res.status },
    }));
  };

  const handleCopy = (id: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1600);
  };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
      {/* Left Sticky Documentation Navigation Sidebar */}
      <aside className="lg:col-span-4 border border-zinc-800 bg-zinc-900/60 rounded-lg p-4 space-y-4 lg:sticky lg:top-20">
        <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
          <div className="flex items-center gap-2 text-xs font-semibold text-white">
            <BookOpen className="w-4 h-4 text-zinc-200" />
            <span>CLIBM v0.1.0 Technical Manual</span>
          </div>
          <span className="text-xs font-mono text-zinc-400 tabular-nums">
            {DOC_CHAPTERS.length} chapters
          </span>
        </div>

        {/* Search Input */}
        <div className="relative">
          <Search className="w-3.5 h-3.5 text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search docs (e.g. BLAKE3, core, diff)..."
            className="w-full pl-8 pr-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white placeholder-zinc-500 focus:outline-none focus:border-zinc-400"
          />
        </div>

        {/* Chapter List */}
        <nav className="space-y-1">
          {filteredChapters.map((chap) => {
            const isSelected = activeChapter.id === chap.id;
            return (
              <button
                key={chap.id}
                type="button"
                onClick={() => setSelectedChapterId(chap.id)}
                className={`w-full text-left px-3 py-2.5 rounded-md transition-colors flex items-start gap-2.5 ${
                  isSelected
                    ? 'bg-zinc-800 border border-zinc-600 text-white'
                    : 'hover:bg-zinc-800/40 text-zinc-300 border border-transparent'
                }`}
              >
                <span className="font-mono text-xs text-zinc-300 font-semibold shrink-0 mt-0.5 tabular-nums">
                  {chap.number}.
                </span>
                <div className="space-y-0.5">
                  <div className="text-xs font-semibold leading-snug">{chap.title}</div>
                  <div className="text-[11px] text-zinc-400 line-clamp-1">{chap.summary}</div>
                </div>
              </button>
            );
          })}
        </nav>

        {/* Quick Package Download Box inside Docs */}
        <div className="pt-3 border-t border-zinc-800 space-y-2">
          <div className="text-[11px] font-mono text-zinc-400">
            Offline Package Downloads
          </div>
          <button
            type="button"
            onClick={downloadCoreOnlyZip}
            className="w-full py-2 px-3 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center justify-center gap-1.5 transition-colors cursor-pointer"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Download Core CLI Only (clibm-core.zip)</span>
          </button>
          <button
            type="button"
            onClick={downloadRustCargoZip}
            className="w-full py-2 px-3 text-xs font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 rounded flex items-center justify-center gap-1.5 transition-colors cursor-pointer"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Download Full Workspace + Plugins (.zip)</span>
          </button>
          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={() => downloadPluginZip('clibm-tui')}
              className="py-1.5 px-2 text-[11px] font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded flex items-center justify-center gap-1 transition-colors cursor-pointer"
            >
              <Download className="w-3 h-3 text-zinc-300" />
              <span>clibm-tui.zip</span>
            </button>
            <button
              type="button"
              onClick={() => downloadPluginZip('clibm-web')}
              className="py-1.5 px-2 text-[11px] font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded flex items-center justify-center gap-1 transition-colors cursor-pointer"
            >
              <Download className="w-3 h-3 text-zinc-300" />
              <span>clibm-web.zip</span>
            </button>
          </div>
        </div>
      </aside>

      {/* Right Main Documentation Content */}
      <article className="lg:col-span-8 border border-zinc-800 bg-zinc-900/50 rounded-lg p-6 md:p-8 space-y-8">
        <div className="border-b border-zinc-800 pb-5 space-y-2">
          <div className="text-xs font-mono text-zinc-400">
            Chapter {activeChapter.number} · CLIBM v0.1.0 Specification &amp; Developer Guide
          </div>
          <h2 className="text-2xl font-bold text-white tracking-tight">
            {activeChapter.number}. {activeChapter.title}
          </h2>
          <p className="text-sm text-zinc-300 leading-relaxed max-w-[72ch]">
            {activeChapter.summary}
          </p>
        </div>

        <div className="space-y-8">
          {activeChapter.sections.map((sec, idx) => (
            <section key={idx} className="space-y-4">
              <h3 className="text-base font-semibold text-white">{sec.heading}</h3>
              <div className="space-y-3 text-sm text-zinc-300 leading-relaxed max-w-[72ch]">
                {sec.body.map((paragraph, pIdx) => (
                  <p key={pIdx}>{paragraph}</p>
                ))}
              </div>

              {sec.codeBlock && (
                <div className="relative bg-zinc-950 border border-zinc-800 rounded-lg p-4">
                  <button
                    type="button"
                    onClick={() => handleCopy(`code-${activeChapter.id}-${idx}`, sec.codeBlock!)}
                    className="absolute top-3 right-3 px-2.5 py-1 text-xs font-mono bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-700 rounded flex items-center gap-1"
                  >
                    {copiedId === `code-${activeChapter.id}-${idx}` ? (
                      <>
                        <Check className="w-3.5 h-3.5 text-white" />
                        <span>Copied</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3.5 h-3.5" />
                        <span>Copy</span>
                      </>
                    )}
                  </button>
                  <pre className="font-mono text-xs text-zinc-200 overflow-x-auto leading-relaxed">
                    {sec.codeBlock}
                  </pre>
                </div>
              )}

              {sec.runnableSnippets && (
                <div className="space-y-3 pt-2">
                  {sec.runnableSnippets.map((snip) => {
                    const executed = snippetOutputs[snip.id];
                    return (
                      <div
                        key={snip.id}
                        className="border border-zinc-800 bg-zinc-950 rounded-lg overflow-hidden"
                      >
                        <div className="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-2">
                          <div className="space-y-0.5">
                            <div className="text-xs font-medium text-white">{snip.label}</div>
                            <div className="text-[11px] text-zinc-400">{snip.explanation}</div>
                          </div>
                          <button
                            type="button"
                            onClick={() => handleRunSnippet(snip)}
                            className="px-3 py-1.5 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                          >
                            <Play className="w-3 h-3" />
                            <span>Run Live in Sandbox</span>
                          </button>
                        </div>

                        <div className="p-3.5 font-mono text-xs space-y-2">
                          <div className="flex items-center gap-2 text-zinc-300">
                            <Terminal className="w-3.5 h-3.5 shrink-0" />
                            <code className="text-white">{snip.command}</code>
                          </div>

                          {executed && (
                            <pre className="mt-2 pt-2 border-t border-zinc-800/80 text-zinc-200 whitespace-pre-wrap overflow-x-auto leading-relaxed">
                              {executed.output}
                            </pre>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </section>
          ))}
        </div>
      </article>
    </div>
  );
};
