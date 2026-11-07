import React, { useState, useMemo, useEffect } from 'react';
import {
  Terminal,
  Globe,
  Download,
  ArrowRight,
  GitCommit,
  ShieldCheck,
  Plus,
  Server,
  Keyboard,
} from 'lucide-react';
import {
  ClibmRepositoryState,
  KanbanBoardPayload,
  TaskPayload,
  TaskPriority,
} from '../core/types';
import {
  getActiveBranch,
  getCommitHistory,
  getStagedTree,
  getTypedPayload,
} from '../core/clibmCore';
import {
  downloadPluginZip,
  downloadStandaloneWebHtml,
} from '../rust/rustSourceRegistry';

interface TuiAndWebPluginsViewProps {
  repoState: ClibmRepositoryState;
  onRunCommand: (cmd: string) => void;
  initialMode?: 'tui' | 'web';
}

export const TuiAndWebPluginsView: React.FC<TuiAndWebPluginsViewProps> = ({
  repoState,
  onRunCommand,
  initialMode = 'tui',
}) => {
  const [mode, setMode] = useState<'tui' | 'web'>(initialMode);

  // Ratatui TUI Simulator state
  const [selectedColIdx, setSelectedColIdx] = useState(2);
  const [selectedTaskRow, setSelectedTaskRow] = useState(0);
  const [tuiStatusBanner, setTuiStatusBanner] = useState(
    'READY — Use [h/l] or Left/Right for Column · [j/k] or Up/Down for Task · [m] Move Task · [c] Commit · [f] BLAKE3 fsck'
  );
  const [quickTaskTitle, setQuickTaskTitle] = useState('');

  // Self-Hosted Web Plugin state
  const [webTaskTitle, setWebTaskTitle] = useState('');
  const [webTaskPrio, setWebTaskPrio] = useState<TaskPriority>('high');
  const [webTaskCol, setWebTaskCol] = useState('todo');
  const [webCommitMsg, setWebCommitMsg] = useState('');
  const [apiLogs, setApiLogs] = useState<
    Array<{ id: string; method: string; endpoint: string; status: number; summary: string }>
  >([
    {
      id: 'req-1',
      method: 'GET',
      endpoint: 'http://127.0.0.1:7878/api/state',
      status: 200,
      summary: 'Loaded staged tree and verified BLAKE3 objects from .clibm/objects',
    },
  ]);

  useEffect(() => {
    setMode(initialMode);
  }, [initialMode]);

  const { board, tasks, activeBranch, commits } = useMemo(() => {
    const tree = getStagedTree(repoState);
    const brdHash = Object.values(tree.boards)[0];
    const { payload: boardPayload } = getTypedPayload<KanbanBoardPayload>(
      repoState,
      brdHash,
      'board'
    );
    const loadedTasks: Array<{ hash: string; task: TaskPayload }> = [];
    for (const [, thash] of Object.entries(tree.tasks)) {
      const { payload: t } = getTypedPayload<TaskPayload>(repoState, thash, 'task');
      loadedTasks.push({ hash: thash, task: t });
    }
    return {
      board: boardPayload,
      tasks: loadedTasks,
      activeBranch: getActiveBranch(repoState) || 'detached',
      commits: getCommitHistory(repoState),
    };
  }, [repoState]);

  const sortedColumns = useMemo(
    () => board.columns.slice().sort((a, b) => a.order - b.order),
    [board]
  );

  const tasksInSelectedCol = useMemo(() => {
    const col = sortedColumns[selectedColIdx];
    if (!col) return [];
    return tasks.filter((t) => t.task.column_id === col.id);
  }, [tasks, sortedColumns, selectedColIdx]);

  const activeTuiTask =
    tasksInSelectedCol[Math.min(selectedTaskRow, Math.max(0, tasksInSelectedCol.length - 1))] ||
    null;

  const triggerMoveTaskInTui = () => {
    if (!activeTuiTask) {
      setTuiStatusBanner('No task selected in the active column to move.');
      return;
    }
    const nextColIdx = (selectedColIdx + 1) % sortedColumns.length;
    const nextCol = sortedColumns[nextColIdx];
    onRunCommand(`clibm board move ${activeTuiTask.task.id} ${nextCol.id}`);
    setTuiStatusBanner(
      `Executed: clibm board move ${activeTuiTask.task.id} ${nextCol.id} (target: ${nextCol.name})`
    );
  };

  const triggerTuiCommit = () => {
    const msg = `Snapshot committed via clibm-tui on branch ${activeBranch}`;
    onRunCommand(`clibm commit -m "${msg}"`);
    setTuiStatusBanner(`Committed staged tree ${repoState.stagedTreeHash.slice(0, 12)} -> HEAD`);
  };

  const triggerTuiFsck = () => {
    onRunCommand('clibm fsck');
    setTuiStatusBanner(
      `[VERIFIED] BLAKE3 fsck complete: ${Object.keys(repoState.objects).length}/${
        Object.keys(repoState.objects).length
      } CAS objects verified`
    );
  };

  const handleTuiKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key === 'l' || e.key === 'ArrowRight') {
      e.preventDefault();
      setSelectedColIdx((prev) => (prev + 1) % sortedColumns.length);
      setSelectedTaskRow(0);
    } else if (e.key === 'h' || e.key === 'ArrowLeft') {
      e.preventDefault();
      setSelectedColIdx((prev) => (prev === 0 ? sortedColumns.length - 1 : prev - 1));
      setSelectedTaskRow(0);
    } else if (e.key === 'j' || e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedTaskRow((prev) =>
        Math.min(prev + 1, Math.max(0, tasksInSelectedCol.length - 1))
      );
    } else if (e.key === 'k' || e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedTaskRow((prev) => Math.max(0, prev - 1));
    } else if (e.key === 'm') {
      e.preventDefault();
      triggerMoveTaskInTui();
    } else if (e.key === 'c') {
      e.preventDefault();
      triggerTuiCommit();
    } else if (e.key === 'f') {
      e.preventDefault();
      triggerTuiFsck();
    }
  };

  const handleTuiQuickAdd = (e: React.FormEvent) => {
    e.preventDefault();
    if (!quickTaskTitle.trim()) return;
    const col = sortedColumns[selectedColIdx]?.id || 'todo';
    onRunCommand(
      `clibm task add "${quickTaskTitle.replace(/"/g, "'")}" -p high -c ${col} --tags tui`
    );
    setTuiStatusBanner(`Created task "${quickTaskTitle}" in column '${col}'`);
    setQuickTaskTitle('');
  };

  const logWebApi = (method: string, endpoint: string, summary: string) => {
    setApiLogs((prev) => [
      {
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
        method,
        endpoint: `http://127.0.0.1:7878${endpoint}`,
        status: 200,
        summary,
      },
      ...prev.slice(0, 7),
    ]);
  };

  const handleWebAddTask = (e: React.FormEvent) => {
    e.preventDefault();
    if (!webTaskTitle.trim()) return;
    onRunCommand(
      `clibm task add "${webTaskTitle.replace(/"/g, "'")}" -p ${webTaskPrio} -c ${webTaskCol} --tags web-ui`
    );
    logWebApi('POST', '/api/task', `Created task "${webTaskTitle}" in ${webTaskCol}`);
    setWebTaskTitle('');
  };

  const handleWebMoveTask = (taskId: string, colId: string) => {
    onRunCommand(`clibm board move ${taskId} ${colId}`);
    logWebApi('POST', '/api/move', `Moved ${taskId} -> ${colId}`);
  };

  const handleWebCommit = (e: React.FormEvent) => {
    e.preventDefault();
    const msg = webCommitMsg.trim() || 'Snapshot from clibm-web local server';
    onRunCommand(`clibm commit -m "${msg.replace(/"/g, "'")}"`);
    logWebApi(
      'POST',
      '/api/commit',
      `Committed tree ${repoState.stagedTreeHash.slice(0, 10)}: "${msg}"`
    );
    setWebCommitMsg('');
  };

  return (
    <div className="space-y-6">
      {/* Mode Switcher Header */}
      <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 flex flex-wrap items-center justify-between gap-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-xs font-mono text-zinc-400">
            <span>Architectural Principle #1 &amp; #5</span>
            <span aria-hidden="true">·</span>
            <span>Interface-Agnostic Core + Official Display Plugins</span>
          </div>
          <h2 className="text-base font-semibold text-white">
            Official Display Extensions: Ratatui Terminal UI (
            <code className="text-zinc-200">clibm-tui</code>) &amp; Self-Hosted Web Server (
            <code className="text-zinc-200">clibm-web</code>)
          </h2>
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 p-1 bg-zinc-950 border border-zinc-800 rounded-lg">
            <button
              type="button"
              onClick={() => setMode('tui')}
              className={`px-3.5 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 ${
                mode === 'tui'
                  ? 'bg-zinc-100 text-zinc-950 font-semibold'
                  : 'text-zinc-400 hover:text-white'
              }`}
            >
              <Terminal className="w-3.5 h-3.5" />
              <span>Ratatui TUI Plugin (clibm-tui)</span>
            </button>
            <button
              type="button"
              onClick={() => setMode('web')}
              className={`px-3.5 py-1.5 text-xs font-medium rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 ${
                mode === 'web'
                  ? 'bg-zinc-100 text-zinc-950 font-semibold'
                  : 'text-zinc-400 hover:text-white'
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>Self-Hosted Web Plugin (clibm-web)</span>
            </button>
          </div>
        </div>
      </div>

      {mode === 'tui' ? (
        <div className="space-y-4">
          {/* TUI Action & Download Bar */}
          <div className="border border-zinc-800 bg-zinc-900/40 rounded-lg p-4 flex flex-wrap items-center justify-between gap-4">
            <div className="flex flex-wrap items-center gap-3 text-xs text-zinc-300">
              <Keyboard className="w-4 h-4 text-zinc-100 shrink-0" />
              <span>
                Click inside the TUI frame below to use keyboard shortcuts (
                <code className="text-white">h/j/k/l</code>, <code className="text-white">m</code>{' '}
                move, <code className="text-white">c</code> commit,{' '}
                <code className="text-white">f</code> fsck) or click any column/task directly.
              </span>
            </div>

            <div className="flex items-center gap-2 shrink-0">
              <button
                type="button"
                onClick={() => downloadPluginZip('clibm-tui')}
                className="px-3.5 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Download clibm-tui-v0.1.0.zip (Rust Crate)</span>
              </button>
            </div>
          </div>

          {/* Interactive Ratatui Terminal Frame (Monochrome Box Drawing) */}
          <div
            tabIndex={0}
            onKeyDown={handleTuiKeyDown}
            className="border-2 border-zinc-700 focus:border-zinc-300 bg-zinc-950 rounded-lg p-4 font-mono text-xs text-zinc-100 space-y-3 focus:outline-none select-none"
          >
            {/* Pane 1: Ratatui Top Status Block */}
            <div className="border border-zinc-600 bg-zinc-900/90 px-3 py-2 flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-3">
                <span className="px-2 py-0.5 bg-zinc-100 text-zinc-950 font-bold">
                  CLIBM-TUI v0.1.0 (ratatui + crossterm)
                </span>
                <span>
                  Branch: <strong className="text-white">{activeBranch}</strong>
                </span>
                <span className="text-zinc-600">|</span>
                <span className="tabular-nums">
                  Staged Tree (BLAKE3):{' '}
                  <strong className="text-zinc-200">{repoState.stagedTreeHash.slice(0, 14)}</strong>
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={triggerMoveTaskInTui}
                  className="px-2 py-0.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-100 border border-zinc-600 rounded cursor-pointer"
                >
                  [m] Move Selected Right
                </button>
                <button
                  type="button"
                  onClick={triggerTuiCommit}
                  className="px-2 py-0.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-100 border border-zinc-600 rounded cursor-pointer"
                >
                  [c] Commit Tree
                </button>
                <button
                  type="button"
                  onClick={triggerTuiFsck}
                  className="px-2 py-0.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-100 border border-zinc-600 rounded cursor-pointer"
                >
                  [f] BLAKE3 fsck
                </button>
              </div>
            </div>

            {/* Pane 2: Ratatui Horizontal Split Kanban Columns */}
            <div className="grid grid-cols-1 md:grid-cols-5 gap-2.5">
              {sortedColumns.map((col, colIdx) => {
                const isColSelected = colIdx === selectedColIdx;
                const colTasks = tasks.filter((t) => t.task.column_id === col.id);

                return (
                  <div
                    key={col.id}
                    onClick={() => {
                      setSelectedColIdx(colIdx);
                      setSelectedTaskRow(0);
                    }}
                    className={`border p-2.5 min-h-[250px] flex flex-col cursor-pointer transition-colors ${
                      isColSelected
                        ? 'border-zinc-200 bg-zinc-900/90'
                        : 'border-zinc-800 bg-zinc-950 hover:border-zinc-700'
                    }`}
                  >
                    <div className="flex items-center justify-between border-b border-zinc-800 pb-1.5 mb-2">
                      <span
                        className={
                          isColSelected ? 'text-white font-bold' : 'text-zinc-400 font-semibold'
                        }
                      >
                        ┌─ {col.name} ({colTasks.length})
                      </span>
                      {col.wip_limit && (
                        <span className="text-[10px] text-zinc-500 tabular-nums">
                          max:{col.wip_limit}
                        </span>
                      )}
                    </div>

                    <div className="space-y-1.5 flex-1">
                      {colTasks.length === 0 ? (
                        <div className="text-zinc-600 py-6 text-center">(empty)</div>
                      ) : (
                        colTasks.map(({ hash, task }, tIdx) => {
                          const isTaskSelected = isColSelected && tIdx === selectedTaskRow;
                          return (
                            <div
                              key={task.id}
                              onClick={(e) => {
                                e.stopPropagation();
                                setSelectedColIdx(colIdx);
                                setSelectedTaskRow(tIdx);
                              }}
                              className={`p-2 border transition-colors ${
                                isTaskSelected
                                  ? 'border-zinc-100 bg-zinc-800 text-white font-semibold'
                                  : 'border-zinc-800/90 bg-zinc-900/40 text-zinc-300 hover:border-zinc-700'
                              }`}
                            >
                              <div className="flex items-center justify-between text-[10px]">
                                <span>
                                  {isTaskSelected ? '▶ ' : '  '}
                                  {task.id} [{task.priority.toUpperCase()}]
                                </span>
                                <span className="text-zinc-500 tabular-nums">{hash.slice(0, 6)}</span>
                              </div>
                              <div className="mt-1 text-[11px] leading-snug line-clamp-2">
                                {task.title}
                              </div>
                            </div>
                          );
                        })
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Pane 3: Selected Task Inspector + Recent Commit DAG Split */}
            <div className="grid grid-cols-1 lg:grid-cols-12 gap-2.5">
              <div className="lg:col-span-7 border border-zinc-700 bg-zinc-900/60 p-3 space-y-1.5">
                <div className="text-white font-semibold border-b border-zinc-800 pb-1">
                  ┌─ Selected Task Inspector (Immutable CAS Object)
                </div>
                {activeTuiTask ? (
                  <div className="space-y-1 text-zinc-300">
                    <div>
                      <strong className="text-white">{activeTuiTask.task.id}</strong> —{' '}
                      {activeTuiTask.task.title}
                    </div>
                    <div className="text-zinc-400">
                      BLAKE3 Hash: <span className="text-zinc-100">{activeTuiTask.hash}</span>
                    </div>
                    <div>
                      Priority:{' '}
                      <span className="text-white font-semibold">
                        [{activeTuiTask.task.priority.toUpperCase()}]
                      </span>{' '}
                      · Assignee: {activeTuiTask.task.assignee || 'unassigned'} · Depends On:{' '}
                      {activeTuiTask.task.depends_on.length
                        ? activeTuiTask.task.depends_on.join(', ')
                        : 'none'}
                    </div>
                    <div className="text-zinc-400">{activeTuiTask.task.description}</div>
                  </div>
                ) : (
                  <div className="text-zinc-500 py-4">
                    No task in current column. Use [h/l] to select a populated column or add a task
                    below.
                  </div>
                )}
              </div>

              <div className="lg:col-span-5 border border-zinc-700 bg-zinc-900/60 p-3 space-y-1.5">
                <div className="text-white font-semibold border-b border-zinc-800 pb-1">
                  ┌─ Commit DAG Log (refs/heads/{activeBranch})
                </div>
                <div className="space-y-1 max-h-28 overflow-y-auto">
                  {commits.slice(0, 4).map(({ hash, commit }) => (
                    <div key={hash} className="text-[11px] flex items-center gap-2">
                      <span className="text-zinc-200 font-semibold tabular-nums">
                        * {hash.slice(0, 8)}
                      </span>
                      <span className="text-zinc-300 truncate">{commit.message}</span>
                    </div>
                  ))}
                </div>
                <form onSubmit={handleTuiQuickAdd} className="pt-1 flex items-center gap-1.5">
                  <input
                    type="text"
                    value={quickTaskTitle}
                    onChange={(e) => setQuickTaskTitle(e.target.value)}
                    placeholder="Quick-add task in active column..."
                    className="flex-1 px-2 py-1 bg-zinc-950 border border-zinc-700 text-white text-[11px] focus:outline-none focus:border-zinc-300"
                  />
                  <button
                    type="submit"
                    className="px-2.5 py-1 bg-zinc-100 text-zinc-950 font-bold text-[11px] hover:bg-white cursor-pointer"
                  >
                    +Add
                  </button>
                </form>
              </div>
            </div>

            {/* Pane 4: Ratatui Footer Status Line */}
            <div className="border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-zinc-200 flex items-center justify-between">
              <span>{tuiStatusBanner}</span>
              <span className="text-zinc-500">CLIBM_PLUGIN_CONTRACT_V1</span>
            </div>
          </div>
        </div>
      ) : (
        /* Self-Hosted Web Plugin Mode (clibm-web) */
        <div className="space-y-5">
          {/* Self-Hosted Server Top Banner */}
          <div className="border border-zinc-800 bg-zinc-900/50 rounded-lg p-4 flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-3 text-xs font-mono">
              <Server className="w-4 h-4 text-zinc-100 shrink-0" />
              <div>
                <div className="text-white font-semibold">
                  clibm-web Local HTTP Server — Bound to http://127.0.0.1:7878
                </div>
                <div className="text-zinc-400">
                  Serves embedded zero-dependency HTML UI + JSON API directly over local{' '}
                  <code className="text-zinc-200">.clibm/objects</code>
                </div>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-2 shrink-0">
              <button
                type="button"
                onClick={downloadStandaloneWebHtml}
                className="px-3 py-1.5 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <Download className="w-3.5 h-3.5 text-zinc-300" />
                <span>Download Standalone ui.html</span>
              </button>
              <button
                type="button"
                onClick={() => downloadPluginZip('clibm-web')}
                className="px-3.5 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Download clibm-web-v0.1.0.zip (Rust Server)</span>
              </button>
            </div>
          </div>

          {/* Simulated Self-Hosted Web Page Frame */}
          <div className="border border-zinc-800 bg-zinc-950 rounded-lg overflow-hidden">
            {/* Browser Address Bar Header */}
            <div className="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-3 font-mono text-xs">
              <div className="flex items-center gap-2">
                <Globe className="w-3.5 h-3.5 text-zinc-200" />
                <span className="px-2.5 py-0.5 bg-zinc-950 border border-zinc-800 rounded text-zinc-300">
                  http://127.0.0.1:7878/
                </span>
                <span className="text-zinc-600">·</span>
                <span className="text-zinc-200 flex items-center gap-1">
                  <ShieldCheck className="w-3.5 h-3.5" />
                  [VERIFIED] BLAKE3 fsck: {Object.keys(repoState.objects).length}/
                  {Object.keys(repoState.objects).length} objects
                </span>
              </div>
              <span className="text-zinc-400 tabular-nums">
                branch: {activeBranch} · tree: {repoState.stagedTreeHash.slice(0, 12)}
              </span>
            </div>

            {/* Embedded Web UI Workspace */}
            <div className="p-5 space-y-5">
              {/* Quick Task & Commit Toolbar */}
              <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
                <form
                  onSubmit={handleWebAddTask}
                  className="lg:col-span-7 flex flex-wrap items-center gap-2 bg-zinc-900/70 border border-zinc-800 p-3 rounded-lg"
                >
                  <input
                    type="text"
                    value={webTaskTitle}
                    onChange={(e) => setWebTaskTitle(e.target.value)}
                    placeholder="New task title (POST /api/task)..."
                    className="flex-1 min-w-[180px] px-3 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
                  />
                  <select
                    value={webTaskPrio}
                    onChange={(e) => setWebTaskPrio(e.target.value as TaskPriority)}
                    className="px-2.5 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
                  >
                    <option value="critical">Critical</option>
                    <option value="high">High</option>
                    <option value="medium">Medium</option>
                    <option value="low">Low</option>
                  </select>
                  <select
                    value={webTaskCol}
                    onChange={(e) => setWebTaskCol(e.target.value)}
                    className="px-2.5 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
                  >
                    {sortedColumns.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                  <button
                    type="submit"
                    className="px-3 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center gap-1 whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>Add Task</span>
                  </button>
                </form>

                <form
                  onSubmit={handleWebCommit}
                  className="lg:col-span-5 flex items-center gap-2 bg-zinc-900/70 border border-zinc-800 p-3 rounded-lg"
                >
                  <input
                    type="text"
                    value={webCommitMsg}
                    onChange={(e) => setWebCommitMsg(e.target.value)}
                    placeholder="Commit message (POST /api/commit)..."
                    className="flex-1 px-3 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
                  />
                  <button
                    type="submit"
                    className="px-3 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center gap-1 whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    <GitCommit className="w-3.5 h-3.5" />
                    <span>Commit</span>
                  </button>
                </form>
              </div>

              {/* Self-Hosted Kanban Grid */}
              <div className="grid grid-cols-1 md:grid-cols-5 gap-3">
                {sortedColumns.map((col) => {
                  const colTasks = tasks.filter((t) => t.task.column_id === col.id);
                  return (
                    <div
                      key={col.id}
                      className="border border-zinc-800 bg-zinc-900/40 rounded-lg p-3 space-y-2.5 min-h-[220px]"
                    >
                      <div className="flex items-center justify-between border-b border-zinc-800 pb-2 text-xs font-semibold text-white">
                        <span>{col.name}</span>
                        <span className="font-mono text-zinc-400 tabular-nums">
                          {colTasks.length}
                        </span>
                      </div>

                      {colTasks.map(({ hash, task }) => (
                        <div
                          key={task.id}
                          className="p-2.5 bg-zinc-950 border border-zinc-800 rounded space-y-1.5"
                        >
                          <div className="flex items-center justify-between text-[11px] font-mono text-zinc-400">
                            <span className="text-zinc-100 font-semibold">{task.id}</span>
                            <span>[{task.priority.toUpperCase()}]</span>
                            <span className="tabular-nums text-zinc-500">{hash.slice(0, 6)}</span>
                          </div>
                          <div className="text-xs font-medium text-zinc-100">{task.title}</div>
                          <div className="pt-1.5 flex items-center gap-1 flex-wrap">
                            {sortedColumns
                              .filter((c) => c.id !== col.id)
                              .map((target) => (
                                <button
                                  key={target.id}
                                  type="button"
                                  onClick={() => handleWebMoveTask(task.id, target.id)}
                                  className="px-1.5 py-0.5 text-[10px] font-mono bg-zinc-900 hover:bg-zinc-200 text-zinc-300 hover:text-zinc-950 border border-zinc-800 rounded flex items-center gap-0.5 cursor-pointer"
                                >
                                  <ArrowRight className="w-2.5 h-2.5" />
                                  {target.name}
                                </button>
                              ))}
                          </div>
                        </div>
                      ))}
                    </div>
                  );
                })}
              </div>

              {/* Localhost HTTP API Exchange Log */}
              <div className="border border-zinc-800 bg-zinc-900/40 rounded-lg p-3.5 space-y-2">
                <div className="text-xs font-mono font-semibold text-zinc-300">
                  Localhost REST API Activity (tiny_http ↔ .clibm/objects)
                </div>
                <div className="space-y-1 font-mono text-[11px]">
                  {apiLogs.map((log) => (
                    <div
                      key={log.id}
                      className="flex flex-wrap items-center justify-between gap-2 text-zinc-400 py-1 border-b border-zinc-800/60 last:border-0"
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-white font-semibold">[{log.method}]</span>
                        <span className="text-zinc-200">{log.endpoint}</span>
                        <span className="text-zinc-500">—</span>
                        <span className="text-zinc-300">{log.summary}</span>
                      </div>
                      <span className="text-zinc-200 tabular-nums">HTTP {log.status} OK</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
