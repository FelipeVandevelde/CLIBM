import React, { useState, useMemo } from 'react';
import {
  Sliders,
  GitCommit,
  User,
  RotateCcw,
  Terminal,
  Check,
  FileJson,
  ArrowRight,
  FolderGit2,
  Plus,
  RefreshCw,
  Trash2,
  Download,
  Copy,
  FileCode2,
  HardDrive,
  Globe,
  Package,
} from 'lucide-react';
import {
  ClibmLanguage,
  ClibmRepositoryState,
  CommitPayload,
  CoreInstallProfile,
  ProjectPayload,
  TaskPayload,
  WorkspacePayload,
} from '../core/types';
import { getStagedTree, getTypedPayload } from '../core/clibmCore';
import { RUST_PROJECT_FILES } from '../rust/rustSourceRegistry';
import {
  stripUtf8BomForDisplay,
  triggerUtf8BomFileDownload,
} from '../rust/zipBuilder';

interface ConfigurationViewProps {
  repoState: ClibmRepositoryState;
  onRunCommand: (cmd: string) => void;
}

const LIFECYCLE_SCRIPT_PATHS = [
  'install.sh',
  'install.ps1',
  'update.sh',
  'update.ps1',
  'uninstall.sh',
  'uninstall.ps1',
] as const;

const PRESET_INSTALL_PATHS = [
  '~/.local/bin',
  '~/.cargo/bin',
  '~/.clibm-bin',
  '/usr/local/bin',
] as const;

export const ConfigurationView: React.FC<ConfigurationViewProps> = ({
  repoState,
  onRunCommand,
}) => {
  const { config } = repoState;

  const [authorName, setAuthorName] = useState(config.author_name);
  const [authorEmail, setAuthorEmail] = useState(config.author_email);
  const [githubRepoUrl, setGithubRepoUrl] = useState(
    config.github_repo_url || 'https://github.com/clibm-org/clibm.git'
  );
  const [updateBranch, setUpdateBranch] = useState(config.update_branch || 'main');
  const [installPathInput, setInstallPathInput] = useState(
    config.install_path || '~/.local/bin'
  );
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [selectedScriptPath, setSelectedScriptPath] = useState<string>('install.sh');
  const [copiedScript, setCopiedScript] = useState<string | null>(null);

  React.useEffect(() => {
    setAuthorName(config.author_name);
    setAuthorEmail(config.author_email);
    setGithubRepoUrl(config.github_repo_url || 'https://github.com/clibm-org/clibm.git');
    setUpdateBranch(config.update_branch || 'main');
    setInstallPathInput(config.install_path || '~/.local/bin');
  }, [
    config.author_name,
    config.author_email,
    config.github_repo_url,
    config.update_branch,
    config.install_path,
  ]);

  const { tree, sortedWorkspaces, projectsById, tasksByProject, autoCommitsCount } = useMemo(() => {
    const stagedTree = getStagedTree(repoState);

    const loadedWorkspaces: WorkspacePayload[] = [];
    for (const [, wsHash] of Object.entries(stagedTree.workspaces)) {
      const { payload: ws } = getTypedPayload<WorkspacePayload>(repoState, wsHash, 'workspace');
      loadedWorkspaces.push(ws);
    }
    loadedWorkspaces.sort(
      (a, b) =>
        a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id)
    );

    const prjMap: Record<string, ProjectPayload> = {};
    for (const [pid, phash] of Object.entries(stagedTree.projects)) {
      const { payload: p } = getTypedPayload<ProjectPayload>(repoState, phash, 'project');
      prjMap[pid] = p;
    }

    const tCounts: Record<string, number> = {};
    for (const [, thash] of Object.entries(stagedTree.tasks)) {
      const { payload: t } = getTypedPayload<TaskPayload>(repoState, thash, 'task');
      tCounts[t.project_id] = (tCounts[t.project_id] || 0) + 1;
    }

    let autoCount = 0;
    for (const obj of Object.values(repoState.objects)) {
      if (obj.Type === 'commit') {
        const c = obj.Payload as CommitPayload;
        if (c && typeof c.message === 'string' && c.message.startsWith('auto(')) {
          autoCount += 1;
        }
      }
    }

    return {
      tree: stagedTree,
      sortedWorkspaces: loadedWorkspaces,
      projectsById: prjMap,
      tasksByProject: tCounts,
      autoCommitsCount: autoCount,
    };
  }, [repoState]);

  const lifecycleScripts = useMemo(() => {
    return RUST_PROJECT_FILES.filter((f) =>
      (LIFECYCLE_SCRIPT_PATHS as readonly string[]).includes(f.relativePath)
    );
  }, []);

  const activeScript =
    lifecycleScripts.find((s) => s.relativePath === selectedScriptPath) || lifecycleScripts[0];

  const handleToggleConfig = (key: string, currentValue: boolean) => {
    onRunCommand(`clibm config set ${key} ${!currentValue}`);
  };

  const handleSetProfile = (profile: CoreInstallProfile) => {
    onRunCommand(`clibm config set install_profile ${profile}`);
  };

  const handleSetLanguage = (lang: ClibmLanguage) => {
    onRunCommand(`clibm config set language ${lang}`);
  };

  const handleSaveInstallPath = (e: React.FormEvent) => {
    e.preventDefault();
    const cleaned = installPathInput.trim() || '~/.local/bin';
    onRunCommand(`clibm config set install_path "${cleaned}"`);
  };

  const handleSaveIdentity = (e: React.FormEvent) => {
    e.preventDefault();
    if (authorName.trim() && authorName.trim() !== config.author_name) {
      onRunCommand(`clibm config set author_name "${authorName.trim()}"`);
    }
    if (authorEmail.trim() && authorEmail.trim() !== config.author_email) {
      onRunCommand(`clibm config set author_email "${authorEmail.trim()}"`);
    }
  };

  const handleSaveGithubConfig = (e: React.FormEvent) => {
    e.preventDefault();
    if (githubRepoUrl.trim() && githubRepoUrl.trim() !== config.github_repo_url) {
      onRunCommand(`clibm config set github_repo_url "${githubRepoUrl.trim()}"`);
    }
    if (updateBranch.trim() && updateBranch.trim() !== config.update_branch) {
      onRunCommand(`clibm config set update_branch "${updateBranch.trim()}"`);
    }
  };

  const handleCreateWorkspace = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newWorkspaceName.trim()) return;
    onRunCommand(`clibm workspace create "${newWorkspaceName.trim()}"`);
    setNewWorkspaceName('');
  };

  const handleCopyText = (id: string, text: string) => {
    navigator.clipboard.writeText(stripUtf8BomForDisplay(text));
    setCopiedScript(id);
    setTimeout(() => setCopiedScript(null), 1600);
  };

  const handleDownloadScript = (filename: string, content: string) => {
    triggerUtf8BomFileDownload(content, filename);
  };

  const generatedInstallCliCmd = `./install.sh ${
    config.install_profile === 'with-defaults' ? '--with-defaults' : '--pure'
  } --lang ${config.language || 'en'} --auto-commit ${String(config.auto_commit)} --install-path "${
    config.install_path || '~/.local/bin'
  }"`;

  return (
    <div className="space-y-6">
      {/* Top Banner: Pure Core Installation & Automatic Per-Modification Commits */}
      <div className="bg-zinc-900/90 border border-zinc-800 rounded-xl p-5 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-5">
        <div className="space-y-2 max-w-3xl">
          <div className="flex flex-wrap items-center gap-2 text-xs font-mono text-zinc-300">
            <GitCommit className="w-3.5 h-3.5 text-zinc-100" />
            <span className="font-semibold text-white">.clibm/config.json</span>
            <span aria-hidden="true">·</span>
            <span>install_profile = {config.install_profile || 'pure'}</span>
            <span aria-hidden="true">·</span>
            <span>language = {config.language || 'en'}</span>
            <span aria-hidden="true">·</span>
            <span>auto_commit = {String(config.auto_commit)}</span>
            <span aria-hidden="true">·</span>
            <span>install_path = {config.install_path || '~/.local/bin'}</span>
          </div>
          <h2 className="text-lg font-bold text-zinc-100 tracking-tight">
            Core Installation Options, Automatic Commits &amp; Repository Configuration
          </h2>
          <p className="text-xs text-zinc-400 leading-relaxed">
            Configure your <strong className="text-zinc-200">Core Installation Profile</strong>{' '}
            (100% Pure Core without plugins vs Core with the 3 default plugins),{' '}
            <strong className="text-zinc-200">Language</strong>,{' '}
            <strong className="text-zinc-200">Automatic Commits</strong>, and{' '}
            <strong className="text-zinc-200">Machine Storage Path</strong> where{' '}
            <code className="text-zinc-200 font-mono">clibm</code> is stored.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-3 shrink-0">
          <button
            type="button"
            onClick={() => handleToggleConfig('auto_commit', config.auto_commit)}
            className={`px-4 py-2.5 rounded-lg text-xs font-mono font-bold transition-all flex items-center gap-2 border cursor-pointer whitespace-nowrap shrink-0 ${
              config.auto_commit
                ? 'bg-zinc-100 text-zinc-950 border-white hover:bg-white'
                : 'bg-zinc-950 text-zinc-200 border-zinc-700 hover:border-zinc-500'
            }`}
          >
            <GitCommit className="w-4 h-4" />
            {config.auto_commit ? 'Auto-Commit: ENABLED (Default)' : 'Auto-Commit: DISABLED'}
          </button>

          <button
            type="button"
            onClick={() => onRunCommand('clibm config reset')}
            className="px-3.5 py-2.5 rounded-lg text-xs font-mono text-zinc-300 hover:text-white bg-zinc-950 hover:bg-zinc-800 border border-zinc-800 transition-colors flex items-center gap-1.5 cursor-pointer whitespace-nowrap shrink-0"
            title="Reset .clibm/config.json to Pure Core default settings"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Reset Pure Defaults
          </button>
        </div>
      </div>

      {/* Interactive Core Installation Options Wizard (4 User-Requested Installation Choices) */}
      <div className="bg-zinc-900/70 border border-zinc-800 rounded-xl p-6 space-y-5">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 border-b border-zinc-800 pb-4">
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-xs font-mono text-zinc-300">
              <Package className="w-3.5 h-3.5 text-zinc-100" />
              <span className="font-semibold text-white">
                CORE INSTALLATION &amp; SETUP WIZARD (install.sh · install.ps1 · clibm init)
              </span>
            </div>
            <h3 className="text-base font-bold text-white">
              Configure Core Installation Options (Profile, Language, Auto-Commit &amp; Storage Location)
            </h3>
            <p className="text-xs text-zinc-400">
              During installation of the core, CLIBM presents these 4 options so you can choose
              whether to keep the core 100% pure (0 plugins) or pre-install the 3 default plugins
              from <code className="font-mono text-zinc-200">plugins/</code>, select your language,
              configure automatic commits, and choose the directory on your machine where{' '}
              <code className="font-mono text-zinc-200">clibm</code> is stored.
            </p>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <button
              type="button"
              onClick={() =>
                onRunCommand(
                  `clibm init ${
                    config.install_profile === 'with-defaults' ? '--with-defaults' : '--pure'
                  } --lang ${config.language || 'en'} --auto-commit ${String(
                    config.auto_commit
                  )} --install-path "${config.install_path || '~/.local/bin'}"`
                )
              }
              className="px-4 py-2 rounded-lg text-xs font-mono font-bold bg-zinc-100 hover:bg-white text-zinc-950 transition-colors cursor-pointer whitespace-nowrap shrink-0"
            >
              Apply &amp; Re-Initialize Core
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          {/* Option 1: Core Profile (Pure Core vs Core + Default Plugins) */}
          <div className="bg-zinc-950 border border-zinc-800 rounded-lg p-4 space-y-3">
            <div className="flex items-center justify-between text-xs font-mono">
              <span className="font-bold text-white">01. Core Profile (install_profile)</span>
              <span className="text-zinc-400">
                Active: <strong className="text-zinc-100">{config.install_profile || 'pure'}</strong>
              </span>
            </div>
            <p className="text-xs text-zinc-400">
              Choose whether to install the <strong className="text-zinc-200">Pure Core</strong>{' '}
              (0 pre-installed plugins; all plugins stay in <code className="font-mono">plugins/</code>)
              or <strong className="text-zinc-200">Core + Default Plugins</strong> (compiles &amp;
              installs <code className="font-mono">clibm-dep-audit</code>,{' '}
              <code className="font-mono">clibm-export-md</code>, and{' '}
              <code className="font-mono">clibm-workflow-lint</code>).
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1">
              <button
                type="button"
                onClick={() => handleSetProfile('pure')}
                className={`p-3 rounded-lg border text-left transition-colors cursor-pointer ${
                  (config.install_profile || 'pure') === 'pure'
                    ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                    : 'bg-zinc-900/60 text-zinc-300 border-zinc-800 hover:border-zinc-700'
                }`}
              >
                <div className="text-xs font-mono font-bold">Pure Core (Default)</div>
                <div
                  className={`text-[11px] mt-0.5 ${
                    (config.install_profile || 'pure') === 'pure'
                      ? 'text-zinc-800'
                      : 'text-zinc-400'
                  }`}
                >
                  0 built-in plugins · 100% pure engine
                </div>
              </button>

              <button
                type="button"
                onClick={() => handleSetProfile('with-defaults')}
                className={`p-3 rounded-lg border text-left transition-colors cursor-pointer ${
                  config.install_profile === 'with-defaults'
                    ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                    : 'bg-zinc-900/60 text-zinc-300 border-zinc-800 hover:border-zinc-700'
                }`}
              >
                <div className="text-xs font-mono font-bold">Core + Default Plugins</div>
                <div
                  className={`text-[11px] mt-0.5 ${
                    config.install_profile === 'with-defaults'
                      ? 'text-zinc-800'
                      : 'text-zinc-400'
                  }`}
                >
                  Installs dep-audit, export-md &amp; workflow-lint
                </div>
              </button>
            </div>
          </div>

          {/* Option 2: Language Selection */}
          <div className="bg-zinc-950 border border-zinc-800 rounded-lg p-4 space-y-3">
            <div className="flex items-center justify-between text-xs font-mono">
              <div className="flex items-center gap-1.5">
                <Globe className="w-3.5 h-3.5 text-zinc-300" />
                <span className="font-bold text-white">02. Language Selection (language)</span>
              </div>
              <span className="text-zinc-400">
                Active: <strong className="text-zinc-100">{config.language || 'en'}</strong>
              </span>
            </div>
            <p className="text-xs text-zinc-400">
              Select the language used for CLI setup prompts and status summaries.
            </p>
            <div className="grid grid-cols-3 gap-2 pt-1">
              {(
                [
                  { code: 'en', label: 'English (en)', sub: 'Default' },
                  { code: 'pt-BR', label: 'Português (pt-BR)', sub: 'Brasil' },
                  { code: 'es', label: 'Español (es)', sub: 'ES / LatAm' },
                ] as const
              ).map((item) => {
                const active = (config.language || 'en') === item.code;
                return (
                  <button
                    key={item.code}
                    type="button"
                    onClick={() => handleSetLanguage(item.code)}
                    className={`p-2.5 rounded-lg border text-left transition-colors cursor-pointer ${
                      active
                        ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                        : 'bg-zinc-900/60 text-zinc-300 border-zinc-800 hover:border-zinc-700'
                    }`}
                  >
                    <div className="text-xs font-mono font-bold">{item.label}</div>
                    <div className={`text-[11px] ${active ? 'text-zinc-800' : 'text-zinc-400'}`}>
                      {item.sub}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Option 3: Automatic Commits during Installation */}
          <div className="bg-zinc-950 border border-zinc-800 rounded-lg p-4 space-y-3">
            <div className="flex items-center justify-between text-xs font-mono">
              <div className="flex items-center gap-1.5">
                <GitCommit className="w-3.5 h-3.5 text-zinc-300" />
                <span className="font-bold text-white">03. Automatic Commits (auto_commit)</span>
              </div>
              <span className="text-zinc-400">
                Active: <strong className="text-zinc-100">{String(config.auto_commit)}</strong>
              </span>
            </div>
            <p className="text-xs text-zinc-400">
              Determine whether every task mutation, board move, or workspace switch is committed
              automatically (<code className="font-mono">true</code> by default) or staged for
              manual <code className="font-mono">clibm commit</code>.
            </p>
            <div className="grid grid-cols-2 gap-2 pt-1">
              <button
                type="button"
                onClick={() => onRunCommand('clibm config set auto_commit true')}
                className={`p-2.5 rounded-lg border text-left transition-colors cursor-pointer ${
                  config.auto_commit
                    ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                    : 'bg-zinc-900/60 text-zinc-300 border-zinc-800 hover:border-zinc-700'
                }`}
              >
                <div className="text-xs font-mono font-bold">Automatic (Default)</div>
                <div
                  className={`text-[11px] ${config.auto_commit ? 'text-zinc-800' : 'text-zinc-400'}`}
                >
                  auto_commit = true
                </div>
              </button>
              <button
                type="button"
                onClick={() => onRunCommand('clibm config set auto_commit false')}
                className={`p-2.5 rounded-lg border text-left transition-colors cursor-pointer ${
                  !config.auto_commit
                    ? 'bg-zinc-100 text-zinc-950 border-white font-semibold'
                    : 'bg-zinc-900/60 text-zinc-300 border-zinc-800 hover:border-zinc-700'
                }`}
              >
                <div className="text-xs font-mono font-bold">Manual Commits</div>
                <div
                  className={`text-[11px] ${!config.auto_commit ? 'text-zinc-800' : 'text-zinc-400'}`}
                >
                  auto_commit = false
                </div>
              </button>
            </div>
          </div>

          {/* Option 4: Machine Storage / Installation Location */}
          <div className="bg-zinc-950 border border-zinc-800 rounded-lg p-4 space-y-3">
            <div className="flex items-center justify-between text-xs font-mono">
              <div className="flex items-center gap-1.5">
                <HardDrive className="w-3.5 h-3.5 text-zinc-300" />
                <span className="font-bold text-white">04. Machine Storage Location (install_path)</span>
              </div>
              <span className="text-zinc-400 truncate max-w-[180px]">
                {config.install_path || '~/.local/bin'}
              </span>
            </div>
            <p className="text-xs text-zinc-400">
              Specify the directory on your machine where the <code className="font-mono">clibm</code>{' '}
              core binary and installed plugin binaries are stored.
            </p>
            <form onSubmit={handleSaveInstallPath} className="space-y-2">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={installPathInput}
                  onChange={(e) => setInstallPathInput(e.target.value)}
                  placeholder="~/.local/bin or custom machine directory"
                  className="flex-1 bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5 text-xs font-mono text-white focus:outline-none focus:border-zinc-500"
                />
                <button
                  type="submit"
                  className="px-3 py-1.5 rounded-lg text-xs font-mono font-bold bg-zinc-100 text-zinc-950 hover:bg-white transition-colors cursor-pointer whitespace-nowrap shrink-0"
                >
                  Set Location
                </button>
              </div>
              <div className="flex flex-wrap items-center gap-1.5">
                {PRESET_INSTALL_PATHS.map((preset) => (
                  <button
                    key={preset}
                    type="button"
                    onClick={() => {
                      setInstallPathInput(preset);
                      onRunCommand(`clibm config set install_path "${preset}"`);
                    }}
                    className={`px-2 py-0.5 rounded text-[11px] font-mono border transition-colors cursor-pointer ${
                      (config.install_path || '~/.local/bin') === preset
                        ? 'bg-zinc-200 text-zinc-950 border-white font-semibold'
                        : 'bg-zinc-900 text-zinc-400 border-zinc-800 hover:text-zinc-200'
                    }`}
                  >
                    {preset}
                  </button>
                ))}
              </div>
            </form>
          </div>
        </div>

        {/* Live Generated Installer Command Bar */}
        <div className="bg-zinc-950 border border-zinc-800 rounded-lg p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="space-y-0.5 font-mono text-xs">
            <div className="text-[11px] text-zinc-400">
              Non-Interactive Installer Command (matches your 4 selected options above):
            </div>
            <div className="text-zinc-100 break-all">$ {generatedInstallCliCmd}</div>
          </div>
          <button
            type="button"
            onClick={() => handleCopyText('install-cli-cmd', generatedInstallCliCmd)}
            className="px-3 py-1.5 rounded text-xs font-mono bg-zinc-900 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 flex items-center gap-1.5 shrink-0 cursor-pointer"
          >
            {copiedScript === 'install-cli-cmd' ? (
              <Check className="w-3.5 h-3.5 text-white" />
            ) : (
              <Copy className="w-3.5 h-3.5" />
            )}
            <span>Copy Command</span>
          </button>
        </div>
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Engine Toggles, GitHub Update Procedure & Safe Uninstallation */}
        <div className="lg:col-span-7 space-y-6">
          {/* GitHub Update & Uninstallation Lifecycle Panel */}
          <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-5 space-y-5">
            <div className="flex items-center justify-between border-b border-zinc-800/80 pb-3">
              <div className="flex items-center gap-2">
                <RefreshCw className="w-4 h-4 text-zinc-200" />
                <h3 className="text-sm font-bold text-zinc-100 font-mono uppercase tracking-wider">
                  GitHub Update &amp; Safe Uninstallation Procedure
                </h3>
              </div>
              <span className="text-[11px] font-mono text-zinc-400">
                clibm update · clibm uninstall [--purge]
              </span>
            </div>

            {/* 1. GitHub Repository Update Section */}
            <div className="space-y-3 border-b border-zinc-800/80 pb-4">
              <div className="space-y-1">
                <div className="text-xs font-semibold text-white">
                  01. GitHub Repository Update Procedure (<code className="font-mono text-zinc-200">clibm update</code> &amp; <code className="font-mono text-zinc-200">update.sh</code>)
                </div>
                <p className="text-xs text-zinc-400">
                  Pulls the latest commits from the configured GitHub repository via{' '}
                  <code className="font-mono text-zinc-300">git pull</code> or{' '}
                  <code className="font-mono text-zinc-300">cargo install --git &lt;repo&gt; --force</code>{' '}
                  without altering local <code className="font-mono text-zinc-300">.clibm/</code> project data.
                </p>
              </div>

              <form onSubmit={handleSaveGithubConfig} className="grid grid-cols-1 sm:grid-cols-12 gap-3">
                <div className="sm:col-span-7">
                  <label className="block text-[11px] font-mono text-zinc-400 mb-1">
                    github_repo_url
                  </label>
                  <input
                    type="text"
                    value={githubRepoUrl}
                    onChange={(e) => setGithubRepoUrl(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-1.5 text-xs font-mono text-zinc-100 focus:outline-none focus:border-zinc-500"
                    placeholder="https://github.com/clibm-org/clibm.git"
                  />
                </div>
                <div className="sm:col-span-3">
                  <label className="block text-[11px] font-mono text-zinc-400 mb-1">
                    update_branch
                  </label>
                  <input
                    type="text"
                    value={updateBranch}
                    onChange={(e) => setUpdateBranch(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-1.5 text-xs font-mono text-zinc-100 focus:outline-none focus:border-zinc-500"
                    placeholder="main"
                  />
                </div>
                <div className="sm:col-span-2 flex items-end">
                  <button
                    type="submit"
                    className="w-full px-3 py-1.5 rounded-lg text-xs font-mono font-semibold bg-zinc-800 hover:bg-zinc-700 text-white transition-colors cursor-pointer whitespace-nowrap"
                  >
                    Save Remote
                  </button>
                </div>
              </form>

              <div className="flex flex-wrap items-center gap-2 pt-1">
                <button
                  type="button"
                  onClick={() => onRunCommand('clibm update --check')}
                  className="px-3 py-1.5 rounded-lg text-xs font-mono bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 transition-colors cursor-pointer whitespace-nowrap shrink-0"
                >
                  Check for Updates (--check)
                </button>
                <button
                  type="button"
                  onClick={() => onRunCommand('clibm update')}
                  className="px-3.5 py-1.5 rounded-lg text-xs font-mono font-bold bg-zinc-100 hover:bg-white text-zinc-950 transition-colors flex items-center gap-1.5 cursor-pointer whitespace-nowrap shrink-0"
                >
                  <RefreshCw className="w-3.5 h-3.5" />
                  Update Core from GitHub
                </button>
                <button
                  type="button"
                  onClick={() => onRunCommand('clibm update --plugins')}
                  className="px-3 py-1.5 rounded-lg text-xs font-mono bg-zinc-800 hover:bg-zinc-700 text-white border border-zinc-600 transition-colors cursor-pointer whitespace-nowrap shrink-0"
                >
                  Update Core + Plugins (--plugins)
                </button>
              </div>
            </div>

            {/* 2. Uninstallation Procedure Section */}
            <div className="space-y-3">
              <div className="space-y-1">
                <div className="text-xs font-semibold text-white">
                  02. Uninstallation Procedure (<code className="font-mono text-zinc-200">clibm uninstall</code> &amp; <code className="font-mono text-zinc-200">uninstall.sh</code>)
                </div>
                <p className="text-xs text-zinc-400">
                  By default, uninstalling removes the <code className="font-mono text-zinc-300">clibm</code>{' '}
                  core binary and all installed plugin binaries from{' '}
                  <code className="font-mono text-zinc-300">{config.install_path || '~/.local/bin'}</code> while preserving your{' '}
                  <code className="font-mono text-zinc-300">.clibm/</code> project data. Pass{' '}
                  <code className="font-mono text-zinc-200">--purge</code> to also delete{' '}
                  <code className="font-mono text-zinc-300">.clibm/</code> repository data.
                </p>
              </div>

              <div className="flex flex-wrap items-center gap-2.5">
                <button
                  type="button"
                  onClick={() => onRunCommand('clibm uninstall')}
                  className="px-3.5 py-2 rounded-lg text-xs font-mono font-semibold bg-zinc-100 hover:bg-white text-zinc-950 transition-colors flex items-center gap-1.5 cursor-pointer whitespace-nowrap shrink-0"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  Uninstall Binaries &amp; Plugins (Keep .clibm Data)
                </button>
                <button
                  type="button"
                  onClick={() => onRunCommand('clibm uninstall --purge')}
                  className="px-3.5 py-2 rounded-lg text-xs font-mono font-semibold bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 transition-colors flex items-center gap-1.5 cursor-pointer whitespace-nowrap shrink-0"
                >
                  <Trash2 className="w-3.5 h-3.5 text-zinc-400" />
                  Full Uninstall + Purge .clibm (--purge)
                </button>
              </div>
            </div>
          </div>

          {/* Core Engine Behavior Toggles */}
          <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-5 space-y-4">
            <div className="flex items-center justify-between border-b border-zinc-800/80 pb-3">
              <div className="flex items-center gap-2">
                <Sliders className="w-4 h-4 text-zinc-300" />
                <h3 className="text-sm font-bold text-zinc-100 font-mono uppercase tracking-wider">
                  Engine &amp; Policy Switches
                </h3>
              </div>
              <span className="text-[11px] font-mono text-zinc-500">
                CLI: clibm config set &lt;key&gt; &lt;value&gt;
              </span>
            </div>

            <div className="divide-y divide-zinc-800/70">
              {/* Toggle 1: auto_commit */}
              <div className="py-3.5 flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="font-bold text-zinc-100">auto_commit</span>
                    <span aria-hidden="true" className="text-zinc-600">·</span>
                    <span className="text-zinc-400">Default: true</span>
                    <span aria-hidden="true" className="text-zinc-600">·</span>
                    <span className="text-zinc-400 tabular-nums">
                      {autoCommitsCount} auto-commits recorded
                    </span>
                  </div>
                  <p className="text-xs text-zinc-400">
                    Automatically commit each task creation, status transition, edit, comment,
                    deletion, or workspace switch individually without requiring manual{' '}
                    <code className="font-mono text-zinc-300">clibm commit</code>.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => handleToggleConfig('auto_commit', config.auto_commit)}
                  className={`px-3 py-1.5 rounded text-xs font-mono font-bold border transition-colors shrink-0 cursor-pointer ${
                    config.auto_commit
                      ? 'bg-zinc-100 text-zinc-950 border-white'
                      : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:border-zinc-700'
                  }`}
                >
                  {config.auto_commit ? 'ON' : 'OFF'}
                </button>
              </div>

              {/* Toggle 2: enforce_dependencies */}
              <div className="py-3.5 flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="font-bold text-zinc-100">enforce_dependencies</span>
                    <span aria-hidden="true" className="text-zinc-600">·</span>
                    <span className="text-zinc-400">Default: true</span>
                  </div>
                  <p className="text-xs text-zinc-400">
                    Block tasks from moving into <code className="font-mono text-zinc-300">done</code>{' '}
                    until all upstream <code className="font-mono text-zinc-300">depends_on</code>{' '}
                    tasks (including cross-workspace dependencies) are completed.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() =>
                    handleToggleConfig('enforce_dependencies', config.enforce_dependencies)
                  }
                  className={`px-3 py-1.5 rounded text-xs font-mono font-bold border transition-colors shrink-0 cursor-pointer ${
                    config.enforce_dependencies
                      ? 'bg-zinc-100 text-zinc-950 border-white'
                      : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:border-zinc-700'
                  }`}
                >
                  {config.enforce_dependencies ? 'ON' : 'OFF'}
                </button>
              </div>

              {/* Toggle 3: enforce_wip_limits */}
              <div className="py-3.5 flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="font-bold text-zinc-100">enforce_wip_limits</span>
                    <span aria-hidden="true" className="text-zinc-600">·</span>
                    <span className="text-zinc-400">Default: true</span>
                  </div>
                  <p className="text-xs text-zinc-400">
                    Strictly enforce Kanban column Work-In-Progress (WIP) limits when moving tasks
                    or transferring tasks across workspaces.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() =>
                    handleToggleConfig('enforce_wip_limits', config.enforce_wip_limits)
                  }
                  className={`px-3 py-1.5 rounded text-xs font-mono font-bold border transition-colors shrink-0 cursor-pointer ${
                    config.enforce_wip_limits
                      ? 'bg-zinc-100 text-zinc-950 border-white'
                      : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:border-zinc-700'
                  }`}
                >
                  {config.enforce_wip_limits ? 'ON' : 'OFF'}
                </button>
              </div>

              {/* Toggle 4: cross_workspace_dependencies */}
              <div className="py-3.5 flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="font-bold text-zinc-100">cross_workspace_dependencies</span>
                    <span aria-hidden="true" className="text-zinc-600">·</span>
                    <span className="text-zinc-400">Default: true</span>
                  </div>
                  <p className="text-xs text-zinc-400">
                    Allow tasks in one workspace (e.g. <code className="font-mono text-zinc-300">OPS-201</code>)
                    to link dependencies on tasks in another workspace (e.g.{' '}
                    <code className="font-mono text-zinc-300">CLIBM-102</code>).
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() =>
                    handleToggleConfig(
                      'cross_workspace_dependencies',
                      config.cross_workspace_dependencies
                    )
                  }
                  className={`px-3 py-1.5 rounded text-xs font-mono font-bold border transition-colors shrink-0 cursor-pointer ${
                    config.cross_workspace_dependencies
                      ? 'bg-zinc-100 text-zinc-950 border-white'
                      : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:border-zinc-700'
                  }`}
                >
                  {config.cross_workspace_dependencies ? 'ON' : 'OFF'}
                </button>
              </div>

              {/* Default Priority Selector */}
              <div className="pt-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                <div className="space-y-1">
                  <span className="text-xs font-mono font-bold text-zinc-100">
                    default_priority
                  </span>
                  <p className="text-xs text-zinc-400">
                    Default priority assigned when <code className="font-mono text-zinc-300">clibm task add</code>{' '}
                    is invoked without <code className="font-mono text-zinc-300">-p</code>.
                  </p>
                </div>
                <div className="flex items-center gap-1.5">
                  {(['low', 'medium', 'high', 'critical'] as const).map((prio) => (
                    <button
                      key={prio}
                      type="button"
                      onClick={() => onRunCommand(`clibm config set default_priority ${prio}`)}
                      className={`px-2.5 py-1 rounded text-[11px] font-mono uppercase transition-colors border cursor-pointer ${
                        config.default_priority === prio
                          ? 'bg-zinc-100 text-zinc-950 border-white font-bold'
                          : 'bg-zinc-950 text-zinc-400 border-zinc-800 hover:text-zinc-200'
                      }`}
                    >
                      {prio}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Commit Author & Identity */}
          <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-5 space-y-4">
            <div className="flex items-center justify-between border-b border-zinc-800/80 pb-3">
              <div className="flex items-center gap-2">
                <User className="w-4 h-4 text-zinc-300" />
                <h3 className="text-sm font-bold text-zinc-100 font-mono uppercase tracking-wider">
                  Commit Author Identity
                </h3>
              </div>
              <span className="text-[11px] font-mono text-zinc-500">
                Stamped on automatic &amp; manual commits
              </span>
            </div>

            <form onSubmit={handleSaveIdentity} className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label className="block text-[11px] font-mono text-zinc-400 uppercase mb-1.5">
                  author_name
                </label>
                <input
                  type="text"
                  value={authorName}
                  onChange={(e) => setAuthorName(e.target.value)}
                  className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-xs font-mono text-zinc-100 focus:outline-none focus:border-zinc-500"
                  placeholder="linus"
                />
              </div>
              <div>
                <label className="block text-[11px] font-mono text-zinc-400 uppercase mb-1.5">
                  author_email
                </label>
                <input
                  type="email"
                  value={authorEmail}
                  onChange={(e) => setAuthorEmail(e.target.value)}
                  className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-xs font-mono text-zinc-100 focus:outline-none focus:border-zinc-500"
                  placeholder="linus@kernel.org"
                />
              </div>
              <div className="sm:col-span-2 flex justify-end">
                <button
                  type="submit"
                  className="px-4 py-2 rounded-lg text-xs font-mono font-bold bg-zinc-100 text-zinc-950 hover:bg-white transition-colors flex items-center gap-1.5 cursor-pointer"
                >
                  <Check className="w-3.5 h-3.5" />
                  Save Author Identity
                </button>
              </div>
            </form>
          </div>
        </div>

        {/* Right Column: Install, Update & Uninstall Scripts Inspector, Workspaces & Config JSON Preview */}
        <div className="lg:col-span-5 space-y-6">
          {/* Install, Update & Uninstall Scripts Inspector */}
          <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-5 space-y-3.5">
            <div className="flex items-center justify-between border-b border-zinc-800/80 pb-3">
              <div className="flex items-center gap-2">
                <FileCode2 className="w-4 h-4 text-zinc-300" />
                <h3 className="text-sm font-bold text-zinc-100 font-mono uppercase tracking-wider">
                  Install, Update &amp; Uninstall Scripts
                </h3>
              </div>
              <span className="text-[11px] font-mono text-zinc-400">
                Linux &amp; Windows · UTF-8-BOM (EF BB BF)
              </span>
            </div>

            <div className="flex items-center gap-1.5 flex-wrap">
              {lifecycleScripts.map((script) => (
                <button
                  key={script.relativePath}
                  type="button"
                  onClick={() => setSelectedScriptPath(script.relativePath)}
                  className={`px-2.5 py-1 rounded text-xs font-mono transition-colors cursor-pointer whitespace-nowrap shrink-0 ${
                    selectedScriptPath === script.relativePath
                      ? 'bg-zinc-100 text-zinc-950 font-bold'
                      : 'bg-zinc-950 text-zinc-400 border border-zinc-800 hover:text-zinc-200'
                  }`}
                >
                  {script.relativePath}
                </button>
              ))}
            </div>

            {activeScript && (
              <div className="space-y-2.5">
                <div className="flex items-center justify-between gap-2 text-xs text-zinc-400">
                  <span>{activeScript.description}</span>
                  <div className="flex items-center gap-1.5 shrink-0">
                    <button
                      type="button"
                      onClick={() =>
                        handleCopyText(activeScript.relativePath, activeScript.content)
                      }
                      className="px-2 py-1 rounded text-[11px] font-mono bg-zinc-950 hover:bg-zinc-800 text-zinc-200 border border-zinc-800 flex items-center gap-1 cursor-pointer"
                    >
                      {copiedScript === activeScript.relativePath ? (
                        <Check className="w-3 h-3 text-white" />
                      ) : (
                        <Copy className="w-3 h-3" />
                      )}
                      <span>Copy</span>
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        handleDownloadScript(activeScript.relativePath, activeScript.content)
                      }
                      className="px-2 py-1 rounded text-[11px] font-mono bg-zinc-100 hover:bg-white text-zinc-950 font-semibold flex items-center gap-1 cursor-pointer"
                    >
                      <Download className="w-3 h-3" />
                      <span>Download (UTF-8-BOM)</span>
                    </button>
                  </div>
                </div>

                <pre className="bg-zinc-950 border border-zinc-800/90 rounded-lg p-3 text-[11px] font-mono text-zinc-200 overflow-x-auto max-h-60 leading-relaxed">
                  {stripUtf8BomForDisplay(activeScript.content)}
                </pre>
              </div>
            )}
          </div>

          {/* Workspace Selection & Alphabetical Cycling */}
          <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-5 space-y-4">
            <div className="flex items-center justify-between border-b border-zinc-800/80 pb-3">
              <div className="flex items-center gap-2">
                <FolderGit2 className="w-4 h-4 text-zinc-300" />
                <h3 className="text-sm font-bold text-zinc-100 font-mono uppercase tracking-wider">
                  Workspaces (A→Z Order)
                </h3>
              </div>
              <button
                type="button"
                onClick={() => onRunCommand('clibm workspace switch')}
                className="px-2.5 py-1 rounded text-[11px] font-mono bg-zinc-100 text-zinc-950 hover:bg-white font-bold transition-colors flex items-center gap-1 cursor-pointer whitespace-nowrap shrink-0"
                title="Run `clibm workspace switch` without arguments to cycle to next workspace in alphabetical order"
              >
                Next (A→Z)
                <ArrowRight className="w-3 h-3" />
              </button>
            </div>

            <p className="text-xs text-zinc-400 leading-relaxed">
              Running <code className="font-mono text-zinc-200">clibm workspace switch</code> without
              an ID or name automatically advances to the next workspace in alphabetical order.
            </p>

            <div className="space-y-2">
              {sortedWorkspaces.map((ws, idx) => {
                const isActive = ws.id === tree.active_workspace_id;
                const prjId = ws.project_ids[0];
                const prj = prjId ? projectsById[prjId] : undefined;
                const taskCount = prjId ? tasksByProject[prjId] || 0 : 0;

                return (
                  <div
                    key={ws.id}
                    className={`p-3 rounded-lg border flex items-center justify-between gap-3 transition-colors ${
                      isActive
                        ? 'bg-zinc-950 border-zinc-300 text-zinc-100'
                        : 'bg-zinc-950/50 border-zinc-800/80 text-zinc-400 hover:border-zinc-700'
                    }`}
                  >
                    <div className="space-y-0.5">
                      <div className="flex items-center gap-2 text-xs font-mono">
                        <span className="text-zinc-500 tabular-nums">0{idx + 1}.</span>
                        <span className="font-bold text-zinc-100">{ws.name}</span>
                        <span aria-hidden="true" className="text-zinc-600">·</span>
                        <span className="text-zinc-300">{ws.id}</span>
                        {isActive && (
                          <>
                            <span aria-hidden="true" className="text-zinc-600">·</span>
                            <span className="text-white font-bold underline">ACTIVE</span>
                          </>
                        )}
                      </div>
                      <div className="text-[11px] font-mono text-zinc-500">
                        Project: {prj ? `${prj.name} (${prj.code})` : 'N/A'} · {taskCount} tasks
                      </div>
                    </div>

                    {!isActive && (
                      <button
                        type="button"
                        onClick={() => onRunCommand(`clibm workspace switch ${ws.id}`)}
                        className="px-2.5 py-1.5 rounded text-xs font-mono bg-zinc-900 hover:bg-zinc-800 text-zinc-200 border border-zinc-700 transition-colors cursor-pointer"
                      >
                        Switch
                      </button>
                    )}
                  </div>
                );
              })}
            </div>

            <form onSubmit={handleCreateWorkspace} className="pt-2 flex gap-2">
              <input
                type="text"
                value={newWorkspaceName}
                onChange={(e) => setNewWorkspaceName(e.target.value)}
                placeholder="New workspace name (e.g. security-audit)..."
                className="flex-1 bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-1.5 text-xs font-mono text-zinc-100 focus:outline-none focus:border-zinc-500"
              />
              <button
                type="submit"
                className="px-3 py-1.5 rounded-lg text-xs font-mono font-bold bg-zinc-100 text-zinc-950 hover:bg-white transition-colors flex items-center gap-1 cursor-pointer whitespace-nowrap shrink-0"
              >
                <Plus className="w-3.5 h-3.5" />
                Create
              </button>
            </form>
          </div>

          {/* Live .clibm/config.json Inspector */}
          <div className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-5 space-y-3">
            <div className="flex items-center justify-between border-b border-zinc-800/80 pb-2.5">
              <div className="flex items-center gap-2">
                <FileJson className="w-4 h-4 text-zinc-300" />
                <h3 className="text-xs font-bold text-zinc-100 font-mono uppercase tracking-wider">
                  .clibm/config.json
                </h3>
              </div>
              <button
                type="button"
                onClick={() => onRunCommand('clibm config show')}
                className="text-[11px] font-mono text-zinc-400 hover:text-zinc-100 flex items-center gap-1 cursor-pointer"
              >
                <Terminal className="w-3 h-3" />
                clibm config show
              </button>
            </div>

            <pre className="bg-zinc-950 border border-zinc-800/90 rounded-lg p-3.5 text-xs font-mono text-zinc-200 overflow-x-auto leading-relaxed">
              {JSON.stringify(config, null, 2)}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
};
