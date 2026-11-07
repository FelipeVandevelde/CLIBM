import React, { useState, useRef, useEffect } from 'react';
import { Terminal, Play, RotateCcw, CornerDownLeft } from 'lucide-react';
import { TerminalHistoryEntry } from '../core/types';

interface TerminalPaneProps {
  activeBranch: string | null;
  stagedTreeHash: string;
  history: TerminalHistoryEntry[];
  onExecute: (command: string) => void;
  onClear: () => void;
  compact?: boolean;
}

const QUICK_COMMANDS = [
  { label: 'status', cmd: 'clibm status' },
  { label: 'workspace switch (A→Z)', cmd: 'clibm workspace switch' },
  { label: 'plugin new', cmd: 'clibm plugin new sprint-velocity --hooks pre-commit,on-task-move' },
  { label: 'update (GitHub)', cmd: 'clibm update --plugins' },
  { label: 'uninstall (safe)', cmd: 'clibm uninstall' },
  { label: 'config show', cmd: 'clibm config show' },
  { label: 'log', cmd: 'clibm log' },
  { label: 'help', cmd: 'clibm help' },
];

export const TerminalPane: React.FC<TerminalPaneProps> = ({
  activeBranch,
  stagedTreeHash,
  history,
  onExecute,
  onClear,
  compact = false,
}) => {
  const [input, setInput] = useState('');
  const [cmdCursor, setCmdCursor] = useState<number>(-1);
  const outputEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const pastCommands = history.map((h) => h.command).filter(Boolean);

  useEffect(() => {
    outputEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;
    onExecute(input);
    setInput('');
    setCmdCursor(-1);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (pastCommands.length === 0) return;
      const nextIdx = cmdCursor < pastCommands.length - 1 ? cmdCursor + 1 : cmdCursor;
      setCmdCursor(nextIdx);
      setInput(pastCommands[pastCommands.length - 1 - nextIdx] || '');
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (cmdCursor <= 0) {
        setCmdCursor(-1);
        setInput('');
      } else {
        const nextIdx = cmdCursor - 1;
        setCmdCursor(nextIdx);
        setInput(pastCommands[pastCommands.length - 1 - nextIdx] || '');
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      const completions = [
        'clibm status',
        'clibm update',
        'clibm update --check',
        'clibm update --plugins',
        'clibm uninstall',
        'clibm uninstall --purge',
        'clibm plugin new sprint-velocity --hooks pre-commit,on-task-move',
        'clibm plugin list',
        'clibm workspace list',
        'clibm workspace switch',
        'clibm workspace create "',
        'clibm config show',
        'clibm config set auto_commit true',
        'clibm task list --all-workspaces',
        'clibm task transfer CLIBM-103 WS-02',
        'clibm task add "',
        'clibm task show CLIBM-101',
        'clibm task history CLIBM-101',
        'clibm board show',
        'clibm board move CLIBM-103 review',
        'clibm commit -m "',
        'clibm log',
        'clibm branch',
        'clibm checkout feat/plugin-hooks',
        'clibm diff main feat/plugin-hooks',
        'clibm merge feat/plugin-hooks',
        'clibm fsck',
        'clibm plugin run dep-audit',
      ];
      const match = completions.find((c) => c.startsWith(input.trim()));
      if (match) setInput(match);
    }
  };

  return (
    <div className="flex flex-col border border-zinc-800 bg-zinc-950 text-zinc-100 rounded-lg overflow-hidden">
      {/* Terminal Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-4 px-4 py-2.5 bg-zinc-900 border-b border-zinc-800">
        <div className="flex items-center gap-3 text-xs font-mono text-zinc-300">
          <Terminal className="w-4 h-4 text-zinc-100 shrink-0" />
          <span className="font-semibold text-white">clibm-cli v0.1.0 (rust/blake3-cas)</span>
          <span aria-hidden="true" className="text-zinc-600">·</span>
          <span>
            branch: <strong className="text-white">{activeBranch || 'detached'}</strong>
          </span>
          <span aria-hidden="true" className="text-zinc-600">·</span>
          <span className="tabular-nums">tree: {stagedTreeHash.slice(0, 12)}</span>
        </div>

        <div className="flex items-center gap-1.5 flex-wrap">
          {QUICK_COMMANDS.map((q) => (
            <button
              key={q.label}
              type="button"
              onClick={() => onExecute(q.cmd)}
              className="px-2.5 py-1 text-xs font-mono text-zinc-300 bg-zinc-800 hover:bg-zinc-700 hover:text-white rounded transition-colors whitespace-nowrap shrink-0 cursor-pointer"
              title={`Run: ${q.cmd}`}
            >
              <Play className="w-2.5 h-2.5 inline mr-1 text-zinc-200" />
              {q.label}
            </button>
          ))}
          <button
            type="button"
            onClick={onClear}
            className="px-2.5 py-1 text-xs font-mono text-zinc-400 hover:text-white bg-zinc-900 hover:bg-zinc-800 border border-zinc-700 rounded transition-colors whitespace-nowrap shrink-0 cursor-pointer"
          >
            <RotateCcw className="w-3 h-3 inline mr-1" />
            Clear
          </button>
        </div>
      </div>

      {/* Scrollable Output Stream */}
      <div
        onClick={() => inputRef.current?.focus()}
        className={`p-4 font-mono text-xs leading-relaxed overflow-y-auto space-y-3 cursor-text ${
          compact ? 'h-72' : 'h-[430px]'
        }`}
      >
        {history.map((entry) => (
          <div key={entry.id} className="space-y-1">
            {entry.command && (
              <div className="flex items-center gap-2 text-zinc-400">
                <span className="text-zinc-100 font-semibold select-none">
                  clibm@{activeBranch || 'HEAD'}:~$
                </span>
                <span className="text-white font-medium">{entry.command}</span>
                <span className="ml-auto text-[11px] text-zinc-600 tabular-nums select-none">
                  {entry.timestamp.slice(11, 19)}
                </span>
              </div>
            )}
            {entry.output && (
              <pre
                className={`whitespace-pre-wrap break-words pl-3 border-l border-zinc-800 ${
                  entry.status === 'error'
                    ? 'text-white font-semibold underline decoration-zinc-500'
                    : entry.status === 'info'
                    ? 'text-zinc-300'
                    : 'text-zinc-200'
                }`}
              >
                {entry.output}
              </pre>
            )}
          </div>
        ))}
        <div ref={outputEndRef} />
      </div>

      {/* Command Input Form */}
      <form
        onSubmit={handleSubmit}
        className="flex items-center gap-2 px-4 py-2.5 bg-zinc-900/90 border-t border-zinc-800"
      >
        <span className="font-mono text-xs font-semibold text-zinc-100 whitespace-nowrap select-none">
          clibm@{activeBranch || 'HEAD'}:~$
        </span>
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder='Type a command (e.g. clibm task add "Optimize CAS index" -p high -c todo) or press Tab...'
          className="flex-1 bg-transparent font-mono text-xs text-white placeholder-zinc-500 focus:outline-none"
          aria-label="CLIBM CLI Command Input"
        />
        <button
          type="submit"
          className="px-3 py-1.5 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0 cursor-pointer"
        >
          <span>Run</span>
          <CornerDownLeft className="w-3 h-3" />
        </button>
      </form>
    </div>
  );
};
