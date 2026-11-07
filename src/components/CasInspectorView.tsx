import React, { useState, useMemo } from 'react';
import { Database, ShieldCheck, Cpu, Play, CheckCircle2 } from 'lucide-react';
import { CasObject, ClibmRepositoryState } from '../core/types';
import { computeCanonicalObjectHash } from '../core/blake3';
import { buildComputedIndex } from '../core/clibmCore';

interface CasInspectorViewProps {
  repoState: ClibmRepositoryState;
  onRunCommand: (cmd: string) => void;
}

export const CasInspectorView: React.FC<CasInspectorViewProps> = ({
  repoState,
  onRunCommand,
}) => {
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [selectedHash, setSelectedHash] = useState<string>(repoState.stagedTreeHash);
  const [verificationMsg, setVerificationMsg] = useState<string | null>(null);

  const allObjects = useMemo(() => {
    const list = Object.values(repoState.objects);
    return list.sort((a, b) => a.Type.localeCompare(b.Type) || a.Hash.localeCompare(b.Hash));
  }, [repoState]);

  const filteredObjects = useMemo(() => {
    if (typeFilter === 'all') return allObjects;
    return allObjects.filter((o) => o.Type === typeFilter);
  }, [allObjects, typeFilter]);

  const selectedObject: CasObject | undefined =
    repoState.objects[selectedHash] || filteredObjects[0];

  const computedIndex = useMemo(() => buildComputedIndex(repoState), [repoState]);

  const handleVerifySelectedObject = () => {
    if (!selectedObject) return;
    const { hash: recomputed, canonicalEnvelopeJson } = computeCanonicalObjectHash(
      selectedObject.Type,
      selectedObject.Version,
      selectedObject.Payload
    );
    const valid = recomputed === selectedObject.Hash;
    setVerificationMsg(
      valid
        ? `[VERIFIED] Canonical JSON (${canonicalEnvelopeJson.length} bytes) hashes to ${recomputed}`
        : `[MISMATCH] Expected ${selectedObject.Hash}, computed ${recomputed}`
    );
  };

  return (
    <div className="space-y-6">
      {/* Top Summary & Integrity Bar */}
      <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-5 flex flex-wrap items-center justify-between gap-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
            <span>Storage Model: Content-Addressable Storage (CAS)</span>
            <span aria-hidden="true">·</span>
            <span>Hash: BLAKE3 (256-bit)</span>
            <span aria-hidden="true">·</span>
            <span>Format: RFC 8785 Canonical JSON v0.1.0</span>
          </div>
          <h2 className="text-base font-semibold text-white">
            Immutable CAS Object Store, Computed Indices &amp; Plugin Contracts
          </h2>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => onRunCommand('clibm fsck')}
            className="px-3.5 py-2 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded-md flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
          >
            <ShieldCheck className="w-4 h-4" />
            <span>Run Full BLAKE3 fsck ({allObjects.length} objects)</span>
          </button>
        </div>
      </div>

      {/* Main Split: CAS Object Table (Left 5 cols) + Canonical Envelope & Index Inspector (Right 7 cols) */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left: Sharded CAS Objects List */}
        <div className="lg:col-span-5 border border-zinc-800 bg-zinc-900/60 rounded-lg overflow-hidden">
          <div className="p-3.5 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-xs font-semibold text-white">
              <Database className="w-4 h-4 text-zinc-200" />
              <span>.clibm/objects/&lt;shard&gt;/&lt;hash&gt;.json</span>
            </div>

            <div className="flex items-center gap-1 flex-wrap">
              {(['all', 'commit', 'tree', 'task', 'board', 'project', 'workspace'] as const).map(
                (t) => (
                  <button
                    key={t}
                    type="button"
                    onClick={() => setTypeFilter(t)}
                    className={`px-2 py-0.5 text-[11px] font-mono rounded transition-colors ${
                      typeFilter === t
                        ? 'bg-zinc-100 text-zinc-950 font-semibold'
                        : 'bg-zinc-950 text-zinc-400 hover:text-white'
                    }`}
                  >
                    {t}
                  </button>
                )
              )}
            </div>
          </div>

          <div className="divide-y divide-zinc-800/70 max-h-[540px] overflow-y-auto">
            {filteredObjects.map((obj) => {
              const isSelected = selectedObject?.Hash === obj.Hash;
              const shard = obj.Hash.slice(0, 2);
              const rest = obj.Hash.slice(2, 16);
              const payloadRec = obj.Payload as Record<string, unknown>;
              const label =
                (payloadRec.id as string) ||
                (payloadRec.message as string) ||
                `active_project=${payloadRec.active_project_id || ''}`;

              return (
                <button
                  key={obj.Hash}
                  type="button"
                  onClick={() => {
                    setSelectedHash(obj.Hash);
                    setVerificationMsg(null);
                  }}
                  className={`w-full text-left px-4 py-2.5 transition-colors flex items-center justify-between gap-3 font-mono text-xs ${
                    isSelected
                      ? 'bg-zinc-800 text-white'
                      : 'hover:bg-zinc-800/40 text-zinc-300'
                  }`}
                >
                  <div className="truncate space-y-0.5">
                    <div className="flex items-center gap-2">
                      <span className="text-white font-semibold">[{obj.Type}]</span>
                      <span aria-hidden="true" className="text-zinc-600">·</span>
                      <span className="text-zinc-200 truncate">{String(label).slice(0, 36)}</span>
                    </div>
                    <div className="text-[11px] text-zinc-500 tabular-nums">
                      .clibm/objects/{shard}/{rest}...json
                    </div>
                  </div>
                  <span className="text-[11px] text-zinc-200 tabular-nums shrink-0">
                    {obj.Hash.slice(0, 10)}
                  </span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Right: Selected Object Envelope + Computed Indices + Plugin Runner */}
        <div className="lg:col-span-7 space-y-6">
          {selectedObject && (
            <div className="border border-zinc-800 bg-zinc-950 rounded-lg overflow-hidden">
              <div className="px-4 py-3 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-3">
                <div className="font-mono text-xs space-x-2">
                  <span className="text-white font-semibold">Object Envelope (Spec 2.2.2)</span>
                  <span className="text-zinc-500">·</span>
                  <span className="text-zinc-200">Type: {selectedObject.Type}</span>
                  <span className="text-zinc-500">·</span>
                  <span className="text-zinc-400">Version: {selectedObject.Version}</span>
                </div>

                <button
                  type="button"
                  onClick={handleVerifySelectedObject}
                  className="px-3 py-1 text-xs font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold rounded flex items-center gap-1.5 transition-colors whitespace-nowrap shrink-0 cursor-pointer"
                >
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  <span>Recompute &amp; Verify BLAKE3</span>
                </button>
              </div>

              {verificationMsg && (
                <div className="px-4 py-2 bg-zinc-900 border-b border-zinc-700 text-xs font-mono text-white break-all">
                  {verificationMsg}
                </div>
              )}

              <div className="p-4 overflow-x-auto max-h-80">
                <pre className="font-mono text-xs text-zinc-200 leading-relaxed">
                  {JSON.stringify(selectedObject, null, 2)}
                </pre>
              </div>
            </div>
          )}

          {/* Computed Secondary Index View (.clibm/index/computed.json) */}
          <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-4 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-xs font-semibold text-white">
                <Cpu className="w-4 h-4 text-zinc-200" />
                <span>Computed Query Index (.clibm/index/computed.json)</span>
              </div>
              <span className="text-xs font-mono text-zinc-400 tabular-nums">
                tree: {computedIndex.tree_hash.slice(0, 12)}
              </span>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs font-mono">
              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded space-y-1">
                <div className="text-zinc-400">By Priority Index</div>
                {Object.entries(computedIndex.by_priority).map(([prio, ids]) => (
                  <div key={prio} className="text-zinc-200">
                    {prio}: [{ids.join(', ')}]
                  </div>
                ))}
              </div>
              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded space-y-1">
                <div className="text-zinc-400">By Tag Index</div>
                {Object.entries(computedIndex.by_tag).map(([tag, ids]) => (
                  <div key={tag} className="text-zinc-200">
                    #{tag}: [{ids.join(', ')}]
                  </div>
                ))}
              </div>
              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded space-y-1">
                <div className="text-zinc-400">Reverse Dependency Graph</div>
                {Object.entries(computedIndex.blocks_tasks).map(([blocker, deps]) => (
                  <div key={blocker} className="text-zinc-200">
                    {blocker} → blocks [{deps.join(', ')}]
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Extensible Plugin Contract Runner */}
          <div className="border border-zinc-800 bg-zinc-900/60 rounded-lg p-4 space-y-3">
            <div className="text-xs font-semibold text-white">
              Registered Extensions (<code className="text-zinc-200">CLIBM_PLUGIN_CONTRACT_V1</code>)
            </div>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {repoState.plugins.map((plugin) => (
                <div
                  key={plugin.name}
                  className="p-3 bg-zinc-950 border border-zinc-800 rounded flex flex-col justify-between gap-2"
                >
                  <div className="space-y-1">
                    <div className="flex items-center justify-between text-xs font-mono">
                      <span className="font-semibold text-white">{plugin.name}</span>
                      <span className="text-zinc-300">v{plugin.version}</span>
                    </div>
                    <p className="text-[11px] text-zinc-400">{plugin.description}</p>
                  </div>
                  <button
                    type="button"
                    onClick={() => onRunCommand(`clibm plugin run ${plugin.name}`)}
                    className="w-full py-1.5 text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded flex items-center justify-center gap-1 transition-colors cursor-pointer"
                  >
                    <Play className="w-3 h-3" />
                    <span>Run {plugin.name}</span>
                  </button>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
