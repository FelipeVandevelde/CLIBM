import React, { useState, useMemo } from 'react';
import { GitCommit, GitCompare, History, RotateCcw, Undo2 } from 'lucide-react';
import { ClibmRepositoryState, CommitPayload } from '../core/types';
import {
  diffRefs,
  getCommitHistory,
  getTypedPayload,
  historyNote,
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
  const [diffFromRef, setDiffFromRef] = useState('HEAD~1');
  const [diffToRef, setDiffToRef] = useState('HEAD');

  const headCommitHash = resolveHeadCommit(repoState);
  const note = historyNote(repoState);

  // One straight history: HEAD and everything before it, newest first.
  const commitLog = useMemo(() => getCommitHistory(repoState), [repoState]);

  const dirty = useMemo(() => {
    if (!headCommitHash) return true;
    const { payload } = getTypedPayload<CommitPayload>(repoState, headCommitHash, 'commit');
    return payload.tree_hash !== repoState.stagedTreeHash;
  }, [repoState, headCommitHash]);

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

  const refOptions = (
    <>
      <option value="HEAD">HEAD (latest)</option>
      {commitLog.length > 1 && <option value="HEAD~1">HEAD~1 (one before)</option>}
      {commitLog.map((c, i) => (
        <option key={c.hash} value={c.hash.slice(0, 12)}>
          {c.hash.slice(0, 10)} · HEAD~{i} · {c.commit.message.slice(0, 32)}
        </option>
      ))}
    </>
  );

  return (
    <div className="space-y-6">
      {/* Top Action Row: Create Commit & Restore */}
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
            With auto_commit on, every change is already a commit.
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

        {/* Restore Box */}
        <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <Undo2 className="w-4 h-4 text-zinc-200" />
              <span>
                Go Back to an Earlier State (<code className="text-zinc-200">clibm restore</code>)
              </span>
            </div>
            <span className="text-xs font-mono text-white font-semibold tabular-nums">
              HEAD: {headCommitHash?.slice(0, 10) || 'none'}
            </span>
          </div>
          <p className="text-xs text-zinc-400">
            CLIBM keeps one straight history, without branches. <code className="text-zinc-300">Restore</code>{' '}
            on a commit below brings its tasks and boards back as a <em>new</em> commit, so nothing in
            the history is lost and task numbers used since then are never reused.
          </p>
          {note && <p className="text-xs text-zinc-300 border-l border-zinc-600 pl-2">{note}</p>}
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              disabled={commitLog.length < 2}
              onClick={() => onRunCommand('clibm restore HEAD~1')}
              className="px-3.5 py-1.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded whitespace-nowrap shrink-0 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
            >
              Undo latest commit (restore HEAD~1)
            </button>
            <button
              type="button"
              disabled={!dirty}
              onClick={() => onRunCommand('clibm restore HEAD --force')}
              className="px-3.5 py-1.5 text-xs font-mono text-zinc-200 bg-zinc-950 border border-zinc-700 hover:border-zinc-400 rounded whitespace-nowrap shrink-0 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
              title="Discard changes that are not committed yet"
            >
              Discard uncommitted changes
            </button>
          </div>
        </div>
      </div>

      {/* Main Split: Commit History (Left 6 cols) + Structural Diff Inspector (Right 6 cols) */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left: Commit History */}
        <div className="lg:col-span-6 border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 space-y-4">
          <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
            <div className="flex items-center gap-2 text-sm font-semibold text-white">
              <History className="w-4 h-4 text-zinc-200" />
              <span>
                Commit History (<code className="text-zinc-200">clibm log</code>)
              </span>
            </div>
            <span className="text-xs font-mono text-zinc-400 tabular-nums">
              {commitLog.length} commits
            </span>
          </div>

          <div className="space-y-3">
            {commitLog.map(({ hash, commit }, index) => {
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
                      {isHead ? (
                        <span className="text-white font-bold underline">(HEAD)</span>
                      ) : (
                        <span className="text-zinc-500 tabular-nums">HEAD~{index}</span>
                      )}
                    </div>
                    {!isHead && (
                      <button
                        type="button"
                        onClick={() => onRunCommand(`clibm restore ${hash.slice(0, 12)}`)}
                        className="px-2 py-0.5 text-[11px] bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded flex items-center gap-1 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                        title="Bring this project state back as a new commit"
                      >
                        <RotateCcw className="w-3 h-3" />
                        <span>Restore</span>
                      </button>
                    )}
                  </div>

                  <p className="text-sm font-medium text-white">{commit.message}</p>

                  <div className="flex flex-wrap items-center gap-2 text-[11px] font-mono text-zinc-400 tabular-nums">
                    <span>Author: {commit.author}</span>
                    <span aria-hidden="true">·</span>
                    <span>{commit.timestamp.slice(0, 19).replace('T', ' ')}</span>
                    <span aria-hidden="true">·</span>
                    <span>tree: {commit.tree_hash.slice(0, 12)}</span>
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

            <div className="flex flex-wrap items-center gap-2 text-xs font-mono min-w-0 max-w-full">
              <select
                value={diffFromRef}
                onChange={(e) => setDiffFromRef(e.target.value)}
                aria-label="Compare from"
                className="min-w-0 max-w-[12rem] flex-1 px-2.5 py-1 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
              >
                {refOptions}
              </select>
              <span className="text-zinc-500">..</span>
              <select
                value={diffToRef}
                onChange={(e) => setDiffToRef(e.target.value)}
                aria-label="Compare to"
                className="min-w-0 max-w-[12rem] flex-1 px-2.5 py-1 bg-zinc-950 border border-zinc-800 rounded text-zinc-200"
              >
                {refOptions}
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
              {commitLog.length < 2
                ? 'Make a change first: there is only one commit to compare.'
                : 'Select two commits to compare.'}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
