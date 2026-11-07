import React, { useState, useMemo } from 'react';
import { GitBranch, GitCommit, GitMerge, GitCompare, History, RotateCcw } from 'lucide-react';
import { ClibmRepositoryState } from '../core/types';
import {
  diffRefs,
  getActiveBranch,
  getCommitHistory,
  resolveHeadCommit,
} from '../core/clibmCore';

interface VersioningAndDiffViewProps {
  repoState: ClibmRepositoryState;
  onRunCommand: (cmd: string) => void;
}

export const VersioningAndDiffView: React.FC<VersioningAndDiffViewProps> = ({
  repoState,
  onRunCommand,
}) => {
  const [commitMessage, setCommitMessage] = useState('');
  const [newBranchName, setNewBranchName] = useState('');
  const [diffFromRef, setDiffFromRef] = useState('main');
  const [diffToRef, setDiffToRef] = useState('feat/plugin-hooks');

  const activeBranch = getActiveBranch(repoState);
  const headCommitHash = resolveHeadCommit(repoState);
  const branchNames = Object.keys(repoState.branches);

  const commitLog = useMemo(() => {
    const allHashes = new Set<string>();
    const combined: ReturnType<typeof getCommitHistory> = [];
    for (const b of Object.keys(repoState.branches)) {
      for (const item of getCommitHistory(repoState, b)) {
        if (!allHashes.has(item.hash)) {
          allHashes.add(item.hash);
          combined.push(item);
        }
      }
    }
    return combined.sort((a, b) => b.commit.timestamp.localeCompare(a.commit.timestamp));
  }, [repoState]);

  const diffReport = useMemo(() => {
    try {
      return diffRefs(repoState, diffFromRef, diffToRef);
    } catch {
      return null;
    }
  }, [repoState, diffFromRef, diffToRef]);

  const handleCommit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!commitMessage.trim()) return;
    onRunCommand(`clibm commit -m "${commitMessage.replace(/"/g, "'")}"`);
    setCommitMessage('');
  };

  const handleCreateBranch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newBranchName.trim()) return;
    const clean = newBranchName.trim().replace(/\s+/g, '-');
    onRunCommand(`clibm branch ${clean}`);
    setNewBranchName('');
  };

  return (
    <div className="space-y-6">
      {/* Top Action Row: Create Commit & Create/Switch Branch */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Create Commit Box */}
        <form
          onSubmit={handleCommit}
          className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-3"
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <GitCommit className="w-4 h-4 text-zinc-200" />
              <span>
                Create Version Snapshot (<code className="text-zinc-200">clibm commit</code>)
              </span>
            </div>
            <span className="text-xs font-mono text-zinc-400 tabular-nums">
              staged tree: {repoState.stagedTreeHash.slice(0, 12)}
            </span>
          </div>
          <p className="text-xs text-zinc-400">
            Commits record a lightweight <code className="text-zinc-300">TreeSnapshotPayload</code>{' '}
            mapping task and board IDs to their BLAKE3 hashes without duplicating unchanged objects.
          </p>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={commitMessage}
              onChange={(e) => setCommitMessage(e.target.value)}
              placeholder="Commit message (e.g. Complete sprint backlog prioritization)..."
              className="flex-1 px-3 py-2 text-xs bg-zinc-950 border border-zinc-800 rounded text-white focus:outline-none focus:border-zinc-400"
            />
            <button
              type="submit"
              className="px-4 py-2 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded transition-colors whitespace-nowrap shrink-0 cursor-pointer"
            >
              Commit Snapshot
            </button>
          </div>
        </form>

        {/* Branch & Merge Controls */}
        <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <GitBranch className="w-4 h-4 text-zinc-200" />
              <span>
                Branches &amp; Merging (<code className="text-zinc-200">refs/heads/*</code>)
              </span>
            </div>
            <span className="text-xs font-mono text-white font-semibold">
              HEAD: {activeBranch || headCommitHash?.slice(0, 10)}
            </span>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {Object.entries(repoState.branches).map(([branch, commitHash]) => {
              const isCurrent = branch === activeBranch;
              return (
                <div
                  key={branch}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded border text-xs font-mono ${
                    isCurrent
                      ? 'bg-zinc-800 border-zinc-300 text-white font-semibold'
                      : 'bg-zinc-950 border-zinc-800 text-zinc-300'
                  }`}
                >
                  <span>{branch}</span>
                  <span className="text-zinc-500 tabular-nums">({commitHash.slice(0, 8)})</span>
                  {!isCurrent && (
                    <>
                      <button
                        type="button"
                        onClick={() => onRunCommand(`clibm checkout ${branch}`)}
                        className="text-zinc-200 hover:text-white hover:underline ml-1 cursor-pointer"
                      >
                        checkout
                      </button>
                      <span aria-hidden="true" className="text-zinc-700">|</span>
                      <button
                        type="button"
                        onClick={() => onRunCommand(`clibm merge ${branch}`)}
                        className="text-zinc-100 hover:text-white hover:underline flex items-center gap-0.5 cursor-pointer"
                      >
                        <GitMerge className="w-3 h-3" />
                        merge
                      </button>
                    </>
                  )}
                </div>
              );
            })}
          </div>

          <form onSubmit={handleCreateBranch} className="flex items-center gap-2 pt-1">
            <input
              type="text"
              value={newBranchName}
              onChange={(e) => setNewBranchName(e.target.value)}
              placeholder="New branch name (e.g. release/v0.2)..."
              className="flex-1 px-3 py-1.5 text-xs bg-zinc-950 border border-zinc-800 rounded text-white font-mono"
            />
            <button
              type="submit"
              className="px-3.5 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded whitespace-nowrap shrink-0 cursor-pointer"
            >
              Create Branch
            </button>
          </form>
        </div>
      </div>

      {/* Main Split: Commit History DAG (Left 6 cols) + Structural Diff Inspector (Right 6 cols) */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left: Commit History DAG */}
        <div className="lg:col-span-6 border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-4">
          <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <History className="w-4 h-4 text-zinc-200" />
              <span>
                Immutable Commit Graph (<code className="text-zinc-200">clibm log</code>)
              </span>
            </div>
            <span className="text-xs font-mono text-zinc-400 tabular-nums">
              {commitLog.length} commits
            </span>
          </div>

          <div className="space-y-3">
            {commitLog.map(({ hash, commit }) => {
              const pointingBranches = Object.entries(repoState.branches)
                .filter(([, h]) => h === hash)
                .map(([b]) => b);
              const isHead = headCommitHash === hash;

              return (
                <div
                  key={hash}
                  className={`p-4 rounded-lg border space-y-2 ${
                    isHead
                      ? 'bg-zinc-900 border-zinc-300'
                      : 'bg-zinc-950/80 border-zinc-800'
                  }`}
                >
                  <div className="flex flex-wrap items-center justify-between gap-2 text-xs font-mono">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-zinc-100 tabular-nums">
                        commit {hash.slice(0, 14)}
                      </span>
                      {pointingBranches.length > 0 && (
                        <span className="text-zinc-300">
                          [{pointingBranches.join(', ')}]
                        </span>
                      )}
                      {isHead && <span className="text-white font-bold underline">(HEAD)</span>}
                    </div>
                    <button
                      type="button"
                      onClick={() => onRunCommand(`clibm checkout ${hash.slice(0, 12)}`)}
                      className="px-2 py-0.5 text-[11px] bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded flex items-center gap-1 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                      title="Recover this exact project state"
                    >
                      <RotateCcw className="w-3 h-3" />
                      <span>Checkout State</span>
                    </button>
                  </div>

                  <p className="text-sm font-medium text-white">{commit.message}</p>

                  <div className="flex flex-wrap items-center gap-2 text-[11px] font-mono text-zinc-400 tabular-nums">
                    <span>Author: {commit.author}</span>
                    <span aria-hidden="true">·</span>
                    <span>{commit.timestamp.slice(0, 19).replace('T', ' ')}</span>
                    <span aria-hidden="true">·</span>
                    <span>tree: {commit.tree_hash.slice(0, 12)}</span>
                    {commit.parent_hashes.length > 0 && (
                      <>
                        <span aria-hidden="true">·</span>
                        <span>
                          parents: {commit.parent_hashes.map((p) => p.slice(0, 8)).join(', ')}
                        </span>
                      </>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Structural Version Diff Viewer */}
        <div className="lg:col-span-6 border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800 pb-3">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <GitCompare className="w-4 h-4 text-zinc-200" />
              <span>
                Structural Tree Diff (<code className="text-zinc-200">clibm diff</code>)
              </span>
            </div>

            <div className="flex items-center gap-2 text-xs font-mono">
              <select
                value={diffFromRef}
                onChange={(e) => setDiffFromRef(e.target.value)}
                className="px-2.5 py-1 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
              >
                {branchNames.map((b) => (
                  <option key={b} value={b}>
                    branch: {b}
                  </option>
                ))}
                {commitLog.map((c) => (
                  <option key={c.hash} value={c.hash.slice(0, 12)}>
                    commit: {c.hash.slice(0, 10)}
                  </option>
                ))}
              </select>
              <span className="text-zinc-500">..</span>
              <select
                value={diffToRef}
                onChange={(e) => setDiffToRef(e.target.value)}
                className="px-2.5 py-1 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
              >
                {branchNames.map((b) => (
                  <option key={b} value={b}>
                    branch: {b}
                  </option>
                ))}
                {commitLog.map((c) => (
                  <option key={c.hash} value={c.hash.slice(0, 12)}>
                    commit: {c.hash.slice(0, 10)}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {diffReport ? (
            <div className="space-y-3">
              <div className="text-xs font-mono text-zinc-400 flex items-center justify-between bg-zinc-950 px-3 py-2 rounded border border-zinc-800 tabular-nums">
                <span>
                  Base Tree:{' '}
                  <strong className="text-zinc-200">
                    {diffReport.from_tree_hash.slice(0, 14)}
                  </strong>
                </span>
                <span>
                  Target Tree:{' '}
                  <strong className="text-zinc-200">{diffReport.to_tree_hash.slice(0, 14)}</strong>
                </span>
              </div>

              {diffReport.changes.length === 0 ? (
                <div className="py-10 text-center text-xs text-zinc-400 font-mono">
                  No differences between {diffReport.from_ref} and {diffReport.to_ref} (trees share
                  identical BLAKE3 digest).
                </div>
              ) : (
                <div className="space-y-2.5">
                  {diffReport.changes.map((ch) => (
                    <div
                      key={`${ch.entity_type}-${ch.entity_id}`}
                      className="p-3.5 bg-zinc-950 border border-zinc-800 rounded space-y-1.5 font-mono text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-white font-semibold">
                          [{ch.change_kind.toUpperCase()}] {ch.entity_type} :: {ch.entity_id}
                        </span>
                        <span className="text-[11px] text-zinc-500 tabular-nums">
                          {ch.old_hash ? ch.old_hash.slice(0, 8) : '00000000'} →{' '}
                          {ch.new_hash ? ch.new_hash.slice(0, 8) : '00000000'}
                        </span>
                      </div>
                      <ul className="space-y-1 text-zinc-300 pl-3">
                        {ch.details.map((d, i) => (
                          <li key={i}>• {d}</li>
                        ))}
                      </ul>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className="py-8 text-center text-xs text-zinc-500">
              Select two valid references to compare.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
