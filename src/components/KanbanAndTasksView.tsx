import React, { useState, useMemo } from 'react';
import {
  Plus,
  Search,
  ArrowRight,
  GitCommit,
  MessageSquare,
  Link2,
  Trash2,
  AlertTriangle,
  CheckCircle2,
  FolderGit2,
  Layers,
  ArrowRightLeft,
  Copy,
} from 'lucide-react';
import {
  ClibmRepositoryState,
  KanbanBoardPayload,
  ProjectPayload,
  TaskPayload,
  TaskPriority,
  WorkspacePayload,
} from '../core/types';
import {
  getStagedTree,
  getTaskHistory,
  getTypedPayload,
} from '../core/clibmCore';

interface KanbanAndTasksViewProps {
  repoState: ClibmRepositoryState;
  onRunCommand: (cmd: string) => void;
}

export const KanbanAndTasksView: React.FC<KanbanAndTasksViewProps> = ({
  repoState,
  onRunCommand,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [priorityFilter, setPriorityFilter] = useState<string>('all');
  const [workspaceScope, setWorkspaceScope] = useState<'active' | 'all'>('active');
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>('CLIBM-103');
  const [showNewTaskForm, setShowNewTaskForm] = useState(false);
  const [showNewWorkspaceInput, setShowNewWorkspaceInput] = useState(false);
  const [newWorkspaceName, setNewWorkspaceName] = useState('');

  // New task form state
  const [newTitle, setNewTitle] = useState('');
  const [newDesc, setNewDesc] = useState('');
  const [newPriority, setNewPriority] = useState<TaskPriority>(repoState.config.default_priority);
  const [newColumn, setNewColumn] = useState('todo');
  const [newAssignee, setNewAssignee] = useState(repoState.config.author_name);
  const [newTags, setNewTags] = useState('rust,cas');
  const [newDependsOn, setNewDependsOn] = useState('');

  // Task inspector interaction state
  const [commentInput, setCommentInput] = useState('');
  const [dependTargetId, setDependTargetId] = useState('');
  const [transferTargetWs, setTransferTargetWs] = useState('');
  const [newColName, setNewColName] = useState('');

  const {
    tree,
    activeWorkspace,
    sortedWorkspaces,
    board,
    project,
    allTasks,
    projectToWorkspace,
  } = useMemo(() => {
    const stagedTree = getStagedTree(repoState);

    // Load all workspaces sorted alphabetically (A -> Z)
    const loadedWorkspaces: WorkspacePayload[] = [];
    const prjToWs: Record<string, WorkspacePayload> = {};
    for (const [, wsHash] of Object.entries(stagedTree.workspaces)) {
      const { payload: ws } = getTypedPayload<WorkspacePayload>(repoState, wsHash, 'workspace');
      loadedWorkspaces.push(ws);
      for (const pid of ws.project_ids) {
        prjToWs[pid] = ws;
      }
    }
    loadedWorkspaces.sort(
      (a, b) =>
        a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id)
    );

    const currentWs =
      loadedWorkspaces.find((w) => w.id === stagedTree.active_workspace_id) ||
      loadedWorkspaces[0];

    const prjHash =
      stagedTree.projects[stagedTree.active_project_id] ||
      Object.values(stagedTree.projects)[0];
    const { payload: prjPayload } = getTypedPayload<ProjectPayload>(
      repoState,
      prjHash,
      'project'
    );

    const activeBoardId = prjPayload.board_ids[0];
    const brdHash =
      (activeBoardId && stagedTree.boards[activeBoardId]) ||
      Object.values(stagedTree.boards)[0];
    const { payload: boardPayload } = getTypedPayload<KanbanBoardPayload>(
      repoState,
      brdHash,
      'board'
    );

    const loadedTasks: Array<{
      hash: string;
      task: TaskPayload;
      workspace: WorkspacePayload | undefined;
    }> = [];
    for (const [, thash] of Object.entries(stagedTree.tasks)) {
      const { payload: t } = getTypedPayload<TaskPayload>(repoState, thash, 'task');
      loadedTasks.push({
        hash: thash,
        task: t,
        workspace: prjToWs[t.project_id],
      });
    }

    return {
      tree: stagedTree,
      activeWorkspace: currentWs,
      sortedWorkspaces: loadedWorkspaces,
      board: boardPayload,
      project: prjPayload,
      allTasks: loadedTasks,
      projectToWorkspace: prjToWs,
    };
  }, [repoState]);

  const scopedTasks = useMemo(() => {
    if (workspaceScope === 'all') {
      return allTasks;
    }
    return allTasks.filter((t) => t.task.project_id === tree.active_project_id);
  }, [allTasks, workspaceScope, tree.active_project_id]);

  const filteredTasks = useMemo(() => {
    return scopedTasks.filter(({ task, workspace }) => {
      if (priorityFilter !== 'all' && task.priority !== priorityFilter) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesTitle = task.title.toLowerCase().includes(q);
        const matchesId = task.id.toLowerCase().includes(q);
        const matchesTag = task.tags.some((tg) => tg.toLowerCase().includes(q));
        const matchesWs = workspace?.name.toLowerCase().includes(q) || workspace?.id.toLowerCase().includes(q);
        return matchesTitle || matchesId || matchesTag || Boolean(matchesWs);
      }
      return true;
    });
  }, [scopedTasks, priorityFilter, searchQuery]);

  const selectedTaskRecord = useMemo(() => {
    if (!selectedTaskId) return null;
    return allTasks.find((t) => t.task.id === selectedTaskId) || null;
  }, [allTasks, selectedTaskId]);

  const selectedTaskHistory = useMemo(() => {
    if (!selectedTaskId) return [];
    return getTaskHistory(repoState, selectedTaskId);
  }, [repoState, selectedTaskId]);

  const handleCreateTask = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;
    let cmd = `clibm task add "${newTitle.replace(/"/g, "'")}" -p ${newPriority} -c ${newColumn}`;
    if (newDesc.trim()) cmd += ` -d "${newDesc.replace(/"/g, "'")}"`;
    if (newAssignee.trim()) cmd += ` -a "${newAssignee.trim()}"`;
    if (newTags.trim()) cmd += ` --tags ${newTags.replace(/\s+/g, '')}`;
    if (newDependsOn.trim()) cmd += ` --depends-on ${newDependsOn.replace(/\s+/g, '')}`;
    onRunCommand(cmd);
    setNewTitle('');
    setNewDesc('');
    setShowNewTaskForm(false);
  };

  const handleCreateWorkspace = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newWorkspaceName.trim()) return;
    onRunCommand(`clibm workspace create "${newWorkspaceName.trim()}"`);
    setNewWorkspaceName('');
    setShowNewWorkspaceInput(false);
  };

  const handleAddComment = (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedTaskRecord || !commentInput.trim()) return;
    onRunCommand(
      `clibm task comment ${selectedTaskRecord.task.id} "${commentInput.replace(/"/g, "'")}"`
    );
    setCommentInput('');
  };

  const handleAddDependency = (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedTaskRecord || !dependTargetId) return;
    onRunCommand(`clibm task depend ${selectedTaskRecord.task.id} ${dependTargetId}`);
    setDependTargetId('');
  };

  const handleTransferTask = (copyMode: boolean) => {
    if (!selectedTaskRecord || !transferTargetWs) return;
    const cmd = `clibm task transfer ${selectedTaskRecord.task.id} ${transferTargetWs}${
      copyMode ? ' --copy' : ''
    }`;
    onRunCommand(cmd);
    setTransferTargetWs('');
  };

  const handleAddColumn = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newColName.trim()) return;
    const slug = newColName.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-');
    onRunCommand(`clibm board column-add ${slug} "${newColName.trim()}" --wip 5`);
    setNewColName('');
  };

  return (
    <div className="space-y-5">
      {/* Multi-Workspace Selection & Cross-Workspace Coordination Bar */}
      <div className="border border-zinc-800 bg-zinc-900/80 rounded-lg p-4 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-3">
          <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
            <FolderGit2 className="w-4 h-4 text-zinc-100" />
            <span className="uppercase tracking-wider text-zinc-400">Workspace:</span>
          </div>

          {/* Workspace Selector Tabs (Sorted Alphabetically A->Z) */}
          <div className="flex flex-wrap items-center gap-1.5">
            {sortedWorkspaces.map((ws) => {
              const isActive = ws.id === activeWorkspace?.id;
              const wsTaskCount = allTasks.filter((t) => t.workspace?.id === ws.id).length;
              return (
                <button
                  key={ws.id}
                  type="button"
                  onClick={() => {
                    if (!isActive) {
                      onRunCommand(`clibm workspace switch ${ws.id}`);
                    }
                  }}
                  className={`px-3 py-1.5 rounded text-xs font-mono transition-colors flex items-center gap-1.5 border cursor-pointer ${
                    isActive
                      ? 'bg-zinc-100 text-zinc-950 border-white font-bold'
                      : 'bg-zinc-950 text-zinc-300 border-zinc-800 hover:border-zinc-600 hover:text-white'
                  }`}
                >
                  <span>{ws.name}</span>
                  <span
                    className={`text-[10px] px-1 rounded ${
                      isActive ? 'bg-zinc-950/15 text-zinc-950' : 'bg-zinc-900 text-zinc-400'
                    }`}
                  >
                    {ws.id} · {wsTaskCount}
                  </span>
                </button>
              );
            })}

            {/* Alphabetical Next-Workspace Button (`clibm workspace switch` without args) */}
            <button
              type="button"
              onClick={() => onRunCommand('clibm workspace switch')}
              className="px-2.5 py-1.5 rounded text-xs font-mono text-zinc-200 bg-zinc-950 hover:bg-zinc-800 border border-zinc-700 transition-colors flex items-center gap-1 cursor-pointer"
              title="Run `clibm workspace switch` with no ID/name to advance to the next workspace in alphabetical order (A→Z)"
            >
              <span>Next (A→Z)</span>
              <ArrowRight className="w-3 h-3" />
            </button>

            <button
              type="button"
              onClick={() => setShowNewWorkspaceInput(!showNewWorkspaceInput)}
              className="px-2.5 py-1.5 rounded text-xs font-mono text-zinc-400 hover:text-white bg-zinc-950 hover:bg-zinc-900 border border-zinc-800 transition-colors flex items-center gap-1 cursor-pointer"
              title="Create a new workspace"
            >
              <Plus className="w-3 h-3" />
              <span>Workspace</span>
            </button>
          </div>
        </div>

        {/* Scope Switcher: Active Workspace Board vs Unified All-Workspaces Coordinator */}
        <div className="flex flex-wrap items-center gap-2.5">
          <div className="flex items-center gap-1 p-1 bg-zinc-950 border border-zinc-800 rounded-md">
            <button
              type="button"
              onClick={() => setWorkspaceScope('active')}
              className={`px-2.5 py-1 text-xs font-mono rounded transition-colors cursor-pointer ${
                workspaceScope === 'active'
                  ? 'bg-zinc-100 text-zinc-950 font-bold'
                  : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              Active Workspace ({activeWorkspace?.name})
            </button>
            <button
              type="button"
              onClick={() => setWorkspaceScope('all')}
              className={`px-2.5 py-1 text-xs font-mono rounded transition-colors flex items-center gap-1 cursor-pointer ${
                workspaceScope === 'all'
                  ? 'bg-zinc-100 text-zinc-950 font-bold'
                  : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <Layers className="w-3 h-3" />
              All Workspaces ({allTasks.length})
            </button>
          </div>
        </div>
      </div>

      {/* Inline New Workspace Form */}
      {showNewWorkspaceInput && (
        <form
          onSubmit={handleCreateWorkspace}
          className="border border-zinc-800 bg-zinc-900/90 rounded-lg p-3.5 flex flex-wrap items-center justify-between gap-3"
        >
          <div className="text-xs font-mono text-zinc-300">
            Create Workspace (<code className="text-zinc-100">clibm workspace create</code>):
          </div>
          <div className="flex items-center gap-2 flex-1 max-w-md">
            <input
              type="text"
              value={newWorkspaceName}
              onChange={(e) => setNewWorkspaceName(e.target.value)}
              placeholder="e.g. kernel-security or release-engineering"
              className="flex-1 px-3 py-1.5 text-xs font-mono bg-zinc-950 border border-zinc-800 rounded text-white focus:outline-none focus:border-zinc-500"
            />
            <button
              type="submit"
              className="px-3 py-1.5 text-xs font-mono font-bold bg-zinc-100 text-zinc-950 hover:bg-white rounded cursor-pointer"
            >
              Create &amp; Switch
            </button>
            <button
              type="button"
              onClick={() => setShowNewWorkspaceInput(false)}
              className="px-2.5 py-1.5 text-xs font-mono text-zinc-400 hover:text-white"
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {/* Project & Kanban Filter Bar */}
      <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-4 flex flex-wrap items-center justify-between gap-4">
        <div className="space-y-1">
          <div className="flex flex-wrap items-center gap-2 text-xs text-zinc-400 font-mono">
            <span>
              Workspace: <strong className="text-zinc-200">{activeWorkspace?.name}</strong> (
              {activeWorkspace?.id})
            </span>
            <span aria-hidden="true">·</span>
            <span>Project: {project.code}</span>
            <span aria-hidden="true">·</span>
            <span>Board: {board.name}</span>
            <span aria-hidden="true">·</span>
            <span className="tabular-nums">
              {scopedTasks.length} tasks ({workspaceScope === 'all' ? 'all workspaces' : 'active workspace'})
            </span>
            <span aria-hidden="true">·</span>
            <span className="text-zinc-300">
              Auto-Commit: <strong>{repoState.config.auto_commit ? 'ON' : 'OFF'}</strong>
            </span>
          </div>
          <h2 className="text-base font-semibold text-white">
            {workspaceScope === 'all'
              ? 'Unified Multi-Workspace Task Coordinator & Cross-Workspace DAG'
              : `Kanban Board — ${activeWorkspace?.name} (${project.code})`}
          </h2>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          {/* Search Input */}
          <div className="relative">
            <Search className="w-3.5 h-3.5 text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Filter by ID, title, tag, workspace..."
              className="pl-8 pr-3 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded-md text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-zinc-400 w-56"
            />
          </div>

          {/* Priority Filter Segmented Control */}
          <div className="flex items-center gap-1 p-1 bg-zinc-950 border border-zinc-800 rounded-md">
            {(['all', 'critical', 'high', 'medium', 'low'] as const).map((prio) => (
              <button
                key={prio}
                type="button"
                onClick={() => setPriorityFilter(prio)}
                className={`px-2.5 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap shrink-0 cursor-pointer ${
                  priorityFilter === prio
                    ? 'bg-zinc-100 text-zinc-950 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                }`}
              >
                {prio === 'all' ? 'All Priorities' : prio.charAt(0).toUpperCase() + prio.slice(1)}
              </button>
            ))}
          </div>

          <button
            type="button"
            onClick={() => setShowNewTaskForm(!showNewTaskForm)}
            className="px-3.5 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded-md transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0 cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>New Task in {project.code}</span>
          </button>
        </div>
      </div>

      {/* Collapsible New Task CLI Generator Form */}
      {showNewTaskForm && (
        <form
          onSubmit={handleCreateTask}
          className="border border-zinc-800 bg-zinc-900/90 rounded-lg p-5 space-y-4"
        >
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-semibold text-white">
              Create Immutable Task Object in {activeWorkspace?.name} (dispatches{' '}
              <code className="text-zinc-200">clibm task add</code>)
            </h3>
            <button
              type="button"
              onClick={() => setShowNewTaskForm(false)}
              className="text-xs text-zinc-400 hover:text-white"
            >
              Cancel
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="md:col-span-2">
              <label className="block text-xs text-zinc-400 mb-1">Task Title</label>
              <input
                type="text"
                required
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
                placeholder="e.g. Implement zero-copy memory mapped CAS index"
                className="w-full px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white focus:outline-none focus:border-zinc-400"
              />
            </div>

            <div>
              <label className="block text-xs text-zinc-400 mb-1">Priority &amp; Column</label>
              <div className="grid grid-cols-2 gap-2">
                <select
                  value={newPriority}
                  onChange={(e) => setNewPriority(e.target.value as TaskPriority)}
                  className="px-2.5 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
                >
                  <option value="critical">Critical</option>
                  <option value="high">High</option>
                  <option value="medium">Medium</option>
                  <option value="low">Low</option>
                </select>
                <select
                  value={newColumn}
                  onChange={(e) => setNewColumn(e.target.value)}
                  className="px-2.5 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
                >
                  {board.columns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div>
              <label className="block text-xs text-zinc-400 mb-1">Assignee</label>
              <input
                type="text"
                value={newAssignee}
                onChange={(e) => setNewAssignee(e.target.value)}
                className="w-full px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
              />
            </div>

            <div>
              <label className="block text-xs text-zinc-400 mb-1">Tags (comma-separated)</label>
              <input
                type="text"
                value={newTags}
                onChange={(e) => setNewTags(e.target.value)}
                placeholder="storage,blake3,cli"
                className="w-full px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white font-mono"
              />
            </div>

            <div>
              <label className="block text-xs text-zinc-400 mb-1">
                Depends On (Same or Cross-Workspace Task IDs)
              </label>
              <input
                type="text"
                value={newDependsOn}
                onChange={(e) => setNewDependsOn(e.target.value)}
                placeholder="e.g. CLIBM-101,OPS-201"
                className="w-full px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white font-mono"
              />
            </div>

            <div className="md:col-span-3">
              <label className="block text-xs text-zinc-400 mb-1">Description</label>
              <input
                type="text"
                value={newDesc}
                onChange={(e) => setNewDesc(e.target.value)}
                placeholder="Technical specification and acceptance criteria..."
                className="w-full px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
              />
            </div>
          </div>

          <div className="flex justify-end">
            <button
              type="submit"
              className="px-4 py-2 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded transition-colors cursor-pointer"
            >
              Execute clibm task add
            </button>
          </div>
        </form>
      )}

      {/* Main Split: Kanban Columns (Left 8 cols) + Selected Task & History Inspector (Right 4 cols) */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-6 items-start">
        <div className="xl:col-span-8 space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {board.columns
              .slice()
              .sort((a, b) => a.order - b.order)
              .map((col) => {
                const colTasks = filteredTasks.filter((t) => t.task.column_id === col.id);
                const overWip = col.wip_limit !== null && colTasks.length > col.wip_limit;

                return (
                  <div
                    key={col.id}
                    className="border border-zinc-800 bg-zinc-900/40 rounded-lg p-3.5 flex flex-col gap-3 min-h-[260px]"
                  >
                    <div className="flex items-center justify-between border-b border-zinc-800/80 pb-2.5">
                      <span className="text-xs font-semibold text-white">{col.name}</span>
                      <span
                        className={`text-xs font-mono tabular-nums ${
                          overWip ? 'text-white font-bold underline' : 'text-zinc-400'
                        }`}
                      >
                        {colTasks.length}
                        {col.wip_limit ? ` / ${col.wip_limit} max` : ''}
                      </span>
                    </div>

                    <div className="space-y-2.5 flex-1">
                      {colTasks.length === 0 ? (
                        <div className="py-8 text-center text-xs text-zinc-500">
                          No tasks in {col.name}
                        </div>
                      ) : (
                        colTasks.map(({ hash, task, workspace }) => {
                          const isSelected = selectedTaskId === task.id;
                          const prioStyle =
                            task.priority === 'critical'
                              ? 'text-white font-bold underline decoration-zinc-500'
                              : task.priority === 'high'
                              ? 'text-zinc-200 font-semibold'
                              : 'text-zinc-400';

                          return (
                            <div
                              key={task.id}
                              onClick={() => setSelectedTaskId(task.id)}
                              className={`p-3 rounded border transition-colors cursor-pointer space-y-2 ${
                                isSelected
                                  ? 'bg-zinc-800/90 border-zinc-300'
                                  : 'bg-zinc-950/80 border-zinc-800/90 hover:border-zinc-700'
                              }`}
                            >
                              {/* Unboxed Zero-Pill Metadata Line */}
                              <div className="flex items-center justify-between gap-2 text-[11px] font-mono text-zinc-400">
                                <div className="flex items-center gap-1.5 flex-wrap">
                                  <span className="font-semibold text-zinc-100">{task.id}</span>
                                  <span aria-hidden="true">·</span>
                                  <span className={prioStyle}>[{task.priority.toUpperCase()}]</span>
                                  {workspace && (
                                    <>
                                      <span aria-hidden="true">·</span>
                                      <span className="text-zinc-300">
                                        {workspace.id}:{workspace.name}
                                      </span>
                                    </>
                                  )}
                                </div>
                                <span
                                  className="tabular-nums text-zinc-500"
                                  title={`BLAKE3: ${hash}`}
                                >
                                  {hash.slice(0, 8)}
                                </span>
                              </div>

                              <p className="text-xs font-medium text-zinc-100 leading-snug">
                                {task.title}
                              </p>

                              {/* Secondary unboxed metadata */}
                              <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-zinc-400">
                                <span>{task.assignee || 'Unassigned'}</span>
                                {task.tags.length > 0 && (
                                  <>
                                    <span aria-hidden="true">·</span>
                                    <span className="font-mono text-zinc-400">
                                      #{task.tags.join(' #')}
                                    </span>
                                  </>
                                )}
                                {task.depends_on.length > 0 && (
                                  <>
                                    <span aria-hidden="true">·</span>
                                    <span className="font-mono text-zinc-200">
                                      deps: {task.depends_on.join(',')}
                                    </span>
                                  </>
                                )}
                              </div>

                              {/* Quick Move Controls */}
                              <div
                                className="pt-2 border-t border-zinc-800/80 flex items-center justify-between gap-1"
                                onClick={(e) => e.stopPropagation()}
                              >
                                <span className="text-[10px] font-mono text-zinc-500">Move to:</span>
                                <div className="flex items-center gap-1 flex-wrap">
                                  {board.columns
                                    .filter((c) => c.id !== task.column_id)
                                    .map((targetCol) => (
                                      <button
                                        key={targetCol.id}
                                        type="button"
                                        onClick={() =>
                                          onRunCommand(`clibm board move ${task.id} ${targetCol.id}`)
                                        }
                                        className="px-1.5 py-0.5 text-[10px] font-mono text-zinc-300 bg-zinc-900 hover:bg-zinc-200 hover:text-zinc-950 border border-zinc-800 rounded transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                                      >
                                        {targetCol.name}
                                      </button>
                                    ))}
                                </div>
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

          {/* Add Custom Column Bar */}
          <form
            onSubmit={handleAddColumn}
            className="border border-zinc-800 bg-zinc-900/40 rounded-lg p-3 flex items-center justify-between gap-3"
          >
            <span className="text-xs text-zinc-400">
              Customize Kanban Workflow (
              <code className="text-zinc-200">clibm board column-add</code>):
            </span>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={newColName}
                onChange={(e) => setNewColName(e.target.value)}
                placeholder="New column name (e.g. QA Testing)"
                className="px-3 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
              />
              <button
                type="submit"
                className="px-3 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              >
                Add Column
              </button>
            </div>
          </form>
        </div>

        {/* Right Column: Selected Task Details, Cross-Workspace Transfer, Dependencies, Comments & History */}
        <div className="xl:col-span-4 border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-5">
          {selectedTaskRecord ? (
            <>
              <div className="border-b border-zinc-800 pb-4 space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 text-xs font-mono text-zinc-200 flex-wrap">
                    <span className="font-bold text-white">{selectedTaskRecord.task.id}</span>
                    <span aria-hidden="true">·</span>
                    <span>{selectedTaskRecord.task.column_id}</span>
                    <span aria-hidden="true">·</span>
                    <span>[{selectedTaskRecord.task.priority.toUpperCase()}]</span>
                    {selectedTaskRecord.workspace && (
                      <>
                        <span aria-hidden="true">·</span>
                        <span className="text-zinc-300">
                          {selectedTaskRecord.workspace.id} ({selectedTaskRecord.workspace.name})
                        </span>
                      </>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      onRunCommand(`clibm task delete ${selectedTaskRecord.task.id}`);
                      setSelectedTaskId(null);
                    }}
                    className="p-1 text-zinc-400 hover:text-white transition-colors"
                    title="Delete task from working tree"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>

                <h3 className="text-base font-semibold text-white leading-snug">
                  {selectedTaskRecord.task.title}
                </h3>
                <p className="text-xs text-zinc-300 leading-relaxed">
                  {selectedTaskRecord.task.description}
                </p>

                <div className="pt-2 text-[11px] font-mono text-zinc-400 space-y-1">
                  <div>
                    BLAKE3 Object Hash:{' '}
                    <span className="text-zinc-100 break-all">{selectedTaskRecord.hash}</span>
                  </div>
                  <div>
                    Assignee:{' '}
                    <span className="text-zinc-200">
                      {selectedTaskRecord.task.assignee || 'unassigned'}
                    </span>
                    {' · '}Due:{' '}
                    <span className="text-zinc-200">{selectedTaskRecord.task.due_date || '-'}</span>
                  </div>
                </div>
              </div>

              {/* Cross-Workspace Task Coordination: Transfer or Copy */}
              <div className="space-y-2.5 border-b border-zinc-800 pb-4">
                <div className="flex items-center gap-1.5 text-xs font-semibold text-white">
                  <ArrowRightLeft className="w-3.5 h-3.5 text-zinc-200" />
                  <span>Cross-Workspace Task Coordination</span>
                </div>
                <p className="text-[11px] text-zinc-400">
                  Move or clone this task into another workspace via{' '}
                  <code className="font-mono text-zinc-300">clibm task transfer</code>.
                </p>
                <div className="flex items-center gap-2">
                  <select
                    value={transferTargetWs}
                    onChange={(e) => setTransferTargetWs(e.target.value)}
                    className="flex-1 px-2.5 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200 font-mono"
                  >
                    <option value="">Select destination workspace...</option>
                    {sortedWorkspaces
                      .filter((ws) => ws.id !== selectedTaskRecord.workspace?.id)
                      .map((ws) => (
                        <option key={ws.id} value={ws.id}>
                          {ws.id} — {ws.name}
                        </option>
                      ))}
                  </select>
                  <button
                    type="button"
                    disabled={!transferTargetWs}
                    onClick={() => handleTransferTask(false)}
                    className="px-2.5 py-1.5 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white disabled:opacity-40 text-zinc-950 rounded cursor-pointer"
                    title="Move task to target workspace"
                  >
                    Move
                  </button>
                  <button
                    type="button"
                    disabled={!transferTargetWs}
                    onClick={() => handleTransferTask(true)}
                    className="px-2.5 py-1.5 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 disabled:opacity-40 text-white rounded flex items-center gap-1 cursor-pointer"
                    title="Copy task into target workspace (--copy)"
                  >
                    <Copy className="w-3 h-3" />
                    Copy
                  </button>
                </div>
              </div>

              {/* Dependencies Section (Supports Cross-Workspace DAG) */}
              <div className="space-y-2.5 border-b border-zinc-800 pb-4">
                <div className="flex items-center gap-1.5 text-xs font-semibold text-white">
                  <Link2 className="w-3.5 h-3.5 text-zinc-200" />
                  <span>Blocking Dependencies (Cross-Workspace DAG)</span>
                </div>

                {selectedTaskRecord.task.depends_on.length === 0 ? (
                  <p className="text-xs text-zinc-400">No blocking task dependencies.</p>
                ) : (
                  <div className="space-y-1.5">
                    {selectedTaskRecord.task.depends_on.map((depId) => {
                      const depObj = allTasks.find((t) => t.task.id === depId);
                      const isDone = depObj?.task.column_id === 'done';
                      const depWs = depObj
                        ? projectToWorkspace[depObj.task.project_id]
                        : undefined;
                      const isCrossWs =
                        depWs && selectedTaskRecord.workspace && depWs.id !== selectedTaskRecord.workspace.id;
                      return (
                        <div
                          key={depId}
                          className="flex items-center justify-between text-xs font-mono bg-zinc-950 border border-zinc-800 px-3 py-1.5 rounded gap-2"
                        >
                          <div className="truncate text-zinc-200">
                            <span>{depId}</span>
                            {isCrossWs && (
                              <span className="ml-1.5 text-[10px] px-1 py-0.5 rounded bg-zinc-800 text-zinc-200">
                                {depWs.id}:{depWs.name}
                              </span>
                            )}
                            <span className="text-zinc-400 ml-1">
                              {depObj?.task.title.slice(0, 22) || 'Unknown'}...
                            </span>
                          </div>
                          <span className="text-zinc-200 flex items-center gap-1 shrink-0">
                            {isDone ? (
                              <>
                                <CheckCircle2 className="w-3 h-3 text-white" /> [DONE]
                              </>
                            ) : (
                              <>
                                <AlertTriangle className="w-3 h-3 text-zinc-300" /> [
                                {depObj?.task.column_id.toUpperCase()}]
                              </>
                            )}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}

                <form onSubmit={handleAddDependency} className="flex items-center gap-2 pt-1">
                  <select
                    value={dependTargetId}
                    onChange={(e) => setDependTargetId(e.target.value)}
                    className="flex-1 px-2.5 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-zinc-200 font-mono"
                  >
                    <option value="">Select task to depend on...</option>
                    {allTasks
                      .filter(
                        (t) =>
                          t.task.id !== selectedTaskRecord.task.id &&
                          !selectedTaskRecord.task.depends_on.includes(t.task.id) &&
                          (repoState.config.cross_workspace_dependencies ||
                            t.task.project_id === selectedTaskRecord.task.project_id)
                      )
                      .map((t) => (
                        <option key={t.task.id} value={t.task.id}>
                          [{t.workspace?.id}] {t.task.id} — {t.task.title.slice(0, 26)}
                        </option>
                      ))}
                  </select>
                  <button
                    type="submit"
                    className="px-3 py-1.5 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-white rounded whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    Link Dependency
                  </button>
                </form>
              </div>

              {/* Comments Section */}
              <div className="space-y-2.5 border-b border-zinc-800 pb-4">
                <div className="flex items-center gap-1.5 text-xs font-semibold text-white">
                  <MessageSquare className="w-3.5 h-3.5 text-zinc-200" />
                  <span>Comments ({selectedTaskRecord.task.comments.length})</span>
                </div>

                <div className="space-y-2 max-h-36 overflow-y-auto">
                  {selectedTaskRecord.task.comments.map((c) => (
                    <div
                      key={c.id}
                      className="p-2.5 bg-zinc-950 border border-zinc-800 rounded text-xs space-y-1"
                    >
                      <div className="flex items-center justify-between text-[11px] text-zinc-400 font-mono">
                        <span className="text-zinc-200 font-medium">{c.author}</span>
                        <span className="tabular-nums">
                          {c.timestamp.slice(0, 16).replace('T', ' ')}
                        </span>
                      </div>
                      <p className="text-zinc-300">{c.body}</p>
                    </div>
                  ))}
                </div>

                <form onSubmit={handleAddComment} className="flex items-center gap-2">
                  <input
                    type="text"
                    value={commentInput}
                    onChange={(e) => setCommentInput(e.target.value)}
                    placeholder="Add comment (auto-commits new task hash)..."
                    className="flex-1 px-2.5 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-white"
                  />
                  <button
                    type="submit"
                    className="px-3 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded whitespace-nowrap shrink-0 cursor-pointer"
                  >
                    Comment
                  </button>
                </form>
              </div>

              {/* Individual Task Change History Across Commits */}
              <div className="space-y-2.5">
                <div className="flex items-center gap-1.5 text-xs font-semibold text-white">
                  <GitCommit className="w-3.5 h-3.5 text-zinc-200" />
                  <span>
                    Individual Task Change History (<code className="text-zinc-300">task history</code>)
                  </span>
                </div>

                <div className="space-y-2 max-h-48 overflow-y-auto">
                  {selectedTaskHistory.map((h) => (
                    <div
                      key={`${h.commit_hash}-${h.task_object_hash}`}
                      className="p-2.5 bg-zinc-950 border border-zinc-800 rounded text-xs font-mono space-y-1"
                    >
                      <div className="flex items-center justify-between text-[11px] text-zinc-400">
                        <span className="text-zinc-200">commit {h.commit_hash.slice(0, 10)}</span>
                        <ArrowRight className="w-3 h-3 text-zinc-600" />
                        <span className="text-white font-semibold">
                          obj {h.task_object_hash.slice(0, 10)}
                        </span>
                      </div>
                      <div className="text-zinc-300 font-sans text-xs">{h.commit_message}</div>
                      <div className="text-[11px] text-zinc-400">
                        col: {h.task_snapshot.column_id} · prio: {h.task_snapshot.priority} ·
                        comments: {h.task_snapshot.comments.length}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </>
          ) : (
            <div className="py-12 text-center text-xs text-zinc-400">
              Select any task card on the Kanban board to inspect its BLAKE3 object hash,
              cross-workspace transfer controls, dependencies, comments, and commit history.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
