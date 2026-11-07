import React, { useState } from 'react';
import { Download, Copy, Check, FileCode, Terminal, FolderGit2, Package } from 'lucide-react';
import {
  RUST_PROJECT_FILES,
  CORE_ONLY_CARGO_TOML,
  downloadCoreOnlyZip,
  downloadRustCargoZip,
  RustProjectFile,
} from '../rust/rustSourceRegistry';
import {
  stripUtf8BomForDisplay,
  triggerUtf8BomFileDownload,
} from '../rust/zipBuilder';

export const RustPackageView: React.FC = () => {
  const [packageScope, setPackageScope] = useState<'core-only' | 'full-workspace'>('core-only');
  const [selectedFile, setSelectedFile] = useState<RustProjectFile>(RUST_PROJECT_FILES[0]);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [selectedPlatform, setSelectedPlatform] = useState<'linux' | 'windows'>('linux');

  const visibleFiles =
    packageScope === 'core-only'
      ? RUST_PROJECT_FILES.filter((f) => f.isCoreOnly)
      : RUST_PROJECT_FILES;

  const rawDisplayedContent =
    packageScope === 'core-only' && selectedFile.relativePath === 'Cargo.toml'
      ? CORE_ONLY_CARGO_TOML
      : selectedFile.content;
  const displayedContent = stripUtf8BomForDisplay(rawDisplayedContent);

  const copyText = (id: string, text: string) => {
    navigator.clipboard.writeText(stripUtf8BomForDisplay(text));
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1800);
  };

  const downloadSingleFile = (file: RustProjectFile) => {
    const filename = file.relativePath.split('/').pop() || 'file.rs';
    const content =
      packageScope === 'core-only' && file.relativePath === 'Cargo.toml'
        ? CORE_ONLY_CARGO_TOML
        : file.content;
    triggerUtf8BomFileDownload(content, filename);
  };

  const totalLines = visibleFiles.reduce((acc, f) => acc + f.lines, 0);

  const coreOnlyFileCount = RUST_PROJECT_FILES.filter((f) => f.isCoreOnly).length;

  const linuxQuickStart =
    packageScope === 'core-only'
      ? `# Option A — Quick Install Directly from GitHub (No Clone Required):
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash

# Option B — Local Install from Downloaded Core-Only Archive:
unzip clibm-core-v0.1.0.zip && cd clibm-core
./install.sh

# Initialize a project in a new terminal session:
mkdir -p ~/my-project && cd ~/my-project
clibm init --workspace "engineering" --code "CLIBM" --name "CLIBM Core"
clibm task add "Implement BLAKE3 CAS storage" -p critical -c in-progress --tags rust,cas
clibm board show
clibm commit -m "Initial project snapshot"`
      : `# Option A — Quick Install Core + Plugins Directly from GitHub:
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.sh | bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.sh | bash

# Option B — Local Install from Complete Bundle Archive:
unzip clibm-v0.1.0-workspace.zip && cd clibm
./install.sh
(cd plugins/clibm-tui && ./install.sh)
(cd plugins/clibm-web && ./install.sh)

# Open a new terminal and run installed plugins inside any ongoing CLIBM project!
mkdir -p ~/my-project && cd ~/my-project
clibm init --workspace "engineering" --code "CLIBM" --name "Ongoing Project"
clibm plugin list
clibm plugin run tui                 # or run binary directly: clibm-tui
clibm plugin run web -- --port 7878  # or run binary directly: clibm-web --port 7878`;

  const windowsQuickStart =
    packageScope === 'core-only'
      ? `# Option A — Quick Install Directly from GitHub in PowerShell (No Clone Required):
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.ps1 | iex

# Option B — Local Install from Downloaded Core-Only Archive:
Expand-Archive -Path .\\clibm-core-v0.1.0.zip -DestinationPath .
cd clibm-core
.\\install.ps1

# Open a new PowerShell window and initialize an ongoing project anywhere on disk:
mkdir ~\\my-project ; cd ~\\my-project
clibm init --workspace "engineering" --code "CLIBM" --name "CLIBM Core"
clibm task add "Implement BLAKE3 CAS storage" -p critical -c in-progress --tags rust,cas
clibm board show
clibm commit -m "Initial project snapshot"`
      : `# Option A — Quick Install Core + Plugins Directly from GitHub in PowerShell:
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.ps1 | iex
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.ps1 | iex
irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-web/install.ps1 | iex

# Option B — Local Install from Complete Bundle Archive:
Expand-Archive -Path .\\clibm-v0.1.0-workspace.zip -DestinationPath .
cd clibm
.\\install.ps1
Push-Location plugins\\clibm-tui ; .\\install.ps1 ; Pop-Location
Push-Location plugins\\clibm-web ; .\\install.ps1 ; Pop-Location

# Open a new PowerShell window and run installed plugins inside any ongoing CLIBM project!
mkdir ~\\my-project ; cd ~\\my-project
clibm init --workspace "engineering" --code "CLIBM" --name "Ongoing Project"
clibm plugin list
clibm plugin run tui                 # or run binary directly: clibm-tui
clibm plugin run web -- --port 7878  # or run binary directly: clibm-web --port 7878`;

  return (
    <div className="space-y-6">
      {/* Header & Core-Only vs Full Workspace Download Banner */}
      <div className="border border-zinc-800 bg-zinc-900/70 rounded-lg p-6">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-2 max-w-3xl">
            <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
              <span>Rust 2021 Edition</span>
              <span aria-hidden="true">·</span>
              <span>Crate: clibm v0.1.0</span>
              <span aria-hidden="true">·</span>
              <span>Linux &amp; Windows Native Terminal CLI</span>
              <span aria-hidden="true">·</span>
              <span className="tabular-nums">
                {visibleFiles.length} files ({totalLines} LOC)
              </span>
            </div>
            <h2 className="text-xl font-bold tracking-tight text-white">
              Download CLIBM Core CLI (Without Plugins) or Complete Bundle
            </h2>
            <p className="text-sm text-zinc-300 leading-relaxed">
              Choose the <strong className="text-white">Core-Only Package (`clibm-core-v0.1.0.zip`)</strong>{' '}
              for a pure headless engine and CLI binary with zero TUI or HTTP dependencies, or download
              the <strong className="text-white">Complete Bundle (`clibm-v0.1.0-workspace.zip`)</strong>{' '}
              containing the core alongside standalone <code className="text-zinc-200">plugins/clibm-tui</code> and{' '}
              <code className="text-zinc-200">plugins/clibm-web</code> folders (each with its own{' '}
              <code className="text-zinc-200">README.md</code> and install scripts).
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-3 shrink-0">
            <button
              type="button"
              onClick={downloadCoreOnlyZip}
              className="px-4 py-2.5 text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 rounded-lg transition-colors flex items-center gap-2 whitespace-nowrap shrink-0 cursor-pointer"
            >
              <Download className="w-4 h-4" />
              <span>Download Core CLI Only (clibm-core-v0.1.0.zip)</span>
            </button>
            <button
              type="button"
              onClick={downloadRustCargoZip}
              className="px-4 py-2.5 text-xs font-medium bg-zinc-900 hover:bg-zinc-800 text-zinc-100 border border-zinc-700 rounded-lg transition-colors flex items-center gap-2 whitespace-nowrap shrink-0 cursor-pointer"
            >
              <Package className="w-4 h-4 text-zinc-300" />
              <span>Download Complete Bundle + Plugins (.zip)</span>
            </button>
          </div>
        </div>

        {/* Package Scope Toggle (Core-Only vs Full Bundle) */}
        <div className="mt-6 pt-5 border-t border-zinc-800 flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-2">
            <span className="text-xs font-mono text-zinc-400">Inspect Package Scope:</span>
            <div className="flex items-center gap-1 p-1 bg-zinc-950 border border-zinc-800 rounded-md">
              <button
                type="button"
                onClick={() => {
                  setPackageScope('core-only');
                  if (!selectedFile.isCoreOnly) {
                    setSelectedFile(RUST_PROJECT_FILES[0]);
                  }
                }}
                className={`px-3 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap shrink-0 ${
                  packageScope === 'core-only'
                    ? 'bg-zinc-100 text-zinc-950 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                }`}
              >
                Core CLI Only ({coreOnlyFileCount} files · No Plugins)
              </button>
              <button
                type="button"
                onClick={() => setPackageScope('full-workspace')}
                className={`px-3 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap shrink-0 ${
                  packageScope === 'full-workspace'
                    ? 'bg-zinc-100 text-zinc-950 font-semibold'
                    : 'text-zinc-400 hover:text-zinc-200'
                }`}
              >
                Complete Bundle + Standalone Plugin Folders ({RUST_PROJECT_FILES.length} files)
              </button>
            </div>
          </div>

          <div className="flex items-center gap-1 p-1 bg-zinc-950 border border-zinc-800 rounded-md">
            <button
              type="button"
              onClick={() => setSelectedPlatform('linux')}
              className={`px-3 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap shrink-0 ${
                selectedPlatform === 'linux'
                  ? 'bg-zinc-800 text-white'
                  : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              Linux (Bash / Zsh)
            </button>
            <button
              type="button"
              onClick={() => setSelectedPlatform('windows')}
              className={`px-3 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap shrink-0 ${
                selectedPlatform === 'windows'
                  ? 'bg-zinc-800 text-white'
                  : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              Windows (PowerShell)
            </button>
          </div>
        </div>

        {/* Terminal Build Commands */}
        <div className="mt-4 relative bg-zinc-950 border border-zinc-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2 text-xs font-mono text-zinc-400">
            <Terminal className="w-3.5 h-3.5 text-zinc-200" />
            <span>
              {packageScope === 'core-only'
                ? 'Core-Only Terminal Build Guide (Zero Plugin Dependencies)'
                : 'Full Workspace Terminal Build Guide'}
            </span>
          </div>
          <button
            type="button"
            onClick={() =>
              copyText(
                'quickstart',
                selectedPlatform === 'linux' ? linuxQuickStart : windowsQuickStart
              )
            }
            className="absolute top-3 right-3 px-2.5 py-1 text-xs font-mono bg-zinc-900 hover:bg-zinc-800 text-zinc-300 border border-zinc-700 rounded flex items-center gap-1.5 transition-colors"
          >
            {copiedId === 'quickstart' ? (
              <>
                <Check className="w-3.5 h-3.5 text-white" />
                <span>Copied</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                <span>Copy Commands</span>
              </>
            )}
          </button>
          <pre className="font-mono text-xs text-zinc-200 overflow-x-auto leading-relaxed">
            {selectedPlatform === 'linux' ? linuxQuickStart : windowsQuickStart}
          </pre>
        </div>
      </div>

      {/* Rust Source Code Explorer */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-stretch">
        {/* Left File List */}
        <div className="lg:col-span-4 border border-zinc-800 bg-zinc-900/60 rounded-lg overflow-hidden flex flex-col">
          <div className="px-4 py-3 border-b border-zinc-800 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <FolderGit2 className="w-4 h-4 text-zinc-200" />
              <span className="text-xs font-semibold text-white">
                {packageScope === 'core-only'
                  ? 'Core-Only Files (/clibm-core)'
                  : 'All Workspace Files (/clibm)'}
              </span>
            </div>
            <span className="text-xs font-mono text-zinc-400 tabular-nums">
              {visibleFiles.length} files
            </span>
          </div>

          <div className="divide-y divide-zinc-800/70 overflow-y-auto max-h-[600px]">
            {visibleFiles.map((file) => {
              const isSelected = selectedFile.relativePath === file.relativePath;
              return (
                <button
                  key={file.relativePath}
                  type="button"
                  onClick={() => setSelectedFile(file)}
                  className={`w-full text-left px-4 py-3 transition-colors flex flex-col gap-1 ${
                    isSelected
                      ? 'bg-zinc-800 text-white'
                      : 'hover:bg-zinc-800/40 text-zinc-300'
                  }`}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-mono text-xs font-medium text-zinc-100 flex items-center gap-1.5">
                      <FileCode className="w-3.5 h-3.5 shrink-0 text-zinc-400" />
                      {file.relativePath}
                    </span>
                    <span className="font-mono text-[11px] text-zinc-400 tabular-nums">
                      {file.lines} lines
                    </span>
                  </div>
                  <p className="text-xs text-zinc-400 line-clamp-2">{file.description}</p>
                </button>
              );
            })}
          </div>
        </div>

        {/* Right Code Viewer */}
        <div className="lg:col-span-8 border border-zinc-800 bg-zinc-950 rounded-lg overflow-hidden flex flex-col">
          <div className="px-4 py-3 bg-zinc-900 border-b border-zinc-800 flex flex-wrap items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2 text-xs font-mono">
                <span className="font-semibold text-white">
                  {packageScope === 'core-only' ? 'clibm-core' : 'clibm'}/
                  {selectedFile.relativePath}
                </span>
                <span aria-hidden="true" className="text-zinc-600">·</span>
                <span className="text-zinc-400">{selectedFile.category}</span>
                <span aria-hidden="true" className="text-zinc-600">·</span>
                <span className="text-zinc-400">UTF-8-BOM (EF BB BF)</span>
                <span aria-hidden="true" className="text-zinc-600">·</span>
                <span className="text-zinc-400 tabular-nums">
                  {displayedContent.split('\n').length} LOC
                </span>
              </div>
              <p className="text-xs text-zinc-400 mt-0.5">{selectedFile.description}</p>
            </div>

            <div className="flex items-center gap-2 shrink-0">
              <button
                type="button"
                onClick={() => copyText(selectedFile.relativePath, displayedContent)}
                className="px-3 py-1.5 text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
              >
                {copiedId === selectedFile.relativePath ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-white" />
                    <span>Copied</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    <span>Copy File</span>
                  </>
                )}
              </button>
              <button
                type="button"
                onClick={() => downloadSingleFile(selectedFile)}
                className="px-3 py-1.5 text-xs font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold rounded transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Download File (UTF-8-BOM)</span>
              </button>
            </div>
          </div>

          <div className="p-4 overflow-auto max-h-[560px]">
            <pre className="font-mono text-xs text-zinc-200 leading-relaxed tabular-nums">
              {displayedContent.split('\n').map((line, i) => (
                <div key={i} className="table-row hover:bg-zinc-900/60">
                  <span className="table-cell pr-4 text-right text-zinc-600 select-none w-10">
                    {i + 1}
                  </span>
                  <span className="table-cell whitespace-pre">{line}</span>
                </div>
              ))}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
};
