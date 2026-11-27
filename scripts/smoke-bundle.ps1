# ==============================================================================
# Smoke-tests one CLIBM source ZIP on Windows (counterpart of smoke-bundle.sh).
#
#   pwsh scripts/smoke-bundle.ps1 <bundles-dir> <bundle-name>
#
# Unpacks <bundles-dir>/latest/<name>.zip into a temporary folder, runs
# `cargo build --release --locked`, installs it with the bundle's own .ps1
# installers (run under Windows PowerShell 5.1, like `iex (irm ...)` would),
# runs clibm init / status / a task, checks the plugin, then runs the core
# uninstaller with -Purge and checks that nothing is left behind.
#
# USERPROFILE, HOME and CLIBM_HOME point at the temporary folder for every
# installer run. The uninstaller also stops running CLIBM processes and edits
# the user PATH entries CLIBM recorded, so run this on a CI machine, not on a
# workstation with CLIBM in use.
# ==============================================================================
param(
    [Parameter(Mandatory = $true)][string]$BundlesDir,
    [Parameter(Mandatory = $true)][string]$Name
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
$Manifest = Get-Content -Raw (Join-Path $RepoRoot 'rust-clibm/bundles.json') | ConvertFrom-Json
$Bundle = $Manifest.bundles | Where-Object { $_.name -eq $Name }
if (-not $Bundle) { throw "Unknown bundle '$Name'" }
$Zip = Join-Path (Resolve-Path $BundlesDir) "latest/$Name.zip"
if (-not (Test-Path $Zip)) { throw "$Zip not found" }

$Plugin = "$($Bundle.plugin)"
$Folder = "$($Bundle.folder)"
$Standalone = "$($Bundle.standalone)"
$IsAll = $Bundle.members -is [string]
$Smoke = if ($Bundle.smoke) { $Bundle.smoke } else { 'installed' }

$Work = Join-Path ([IO.Path]::GetTempPath()) "clibm-smoke-$Name-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
$HomeDir = Join-Path $Work 'home'
$Bin = Join-Path $Work 'bin'
$Project = Join-Path $Work 'project'
New-Item -ItemType Directory -Force $HomeDir, $Bin, $Project | Out-Null

$RealCargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$RealRustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
$env:CARGO_HOME = $RealCargoHome
$env:RUSTUP_HOME = $RealRustupHome

function Write-Step([string]$Text) { Write-Host ""; Write-Host "==> [$Name] $Text" }
function Fail([string]$Text) { throw "SMOKE FAILED [$Name]: $Text" }

# Runs a native command and fails on a non-zero exit code.
function Invoke-Checked {
    param([string]$File, [string[]]$Arguments, [string]$Cwd = (Get-Location).Path)
    Push-Location $Cwd
    try {
        & $File @Arguments
        if ($LASTEXITCODE -ne 0) { Fail "$File $($Arguments -join ' ') exited with $LASTEXITCODE" }
    } finally { Pop-Location }
}

# Runs an installer script in a fresh Windows PowerShell with the temporary profile.
function Invoke-Installer {
    param([string]$Script, [string[]]$Arguments, [string]$Cwd, [switch]$IsolatedCargo)
    $saved = @{ USERPROFILE = $env:USERPROFILE; HOME = $env:HOME; CLIBM_HOME = $env:CLIBM_HOME; CARGO_HOME = $env:CARGO_HOME; RUSTUP_HOME = $env:RUSTUP_HOME }
    try {
        $env:USERPROFILE = $HomeDir; $env:HOME = $HomeDir; $env:CLIBM_HOME = $HomeDir
        if ($IsolatedCargo) {
            $env:CARGO_HOME = Join-Path $HomeDir '.cargo'; $env:RUSTUP_HOME = Join-Path $HomeDir '.rustup'
        }
        Invoke-Checked -File 'powershell.exe' -Arguments (@('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $Script) + $Arguments) -Cwd $Cwd
    } finally {
        foreach ($k in $saved.Keys) { Set-Item -Path "env:$k" -Value $saved[$k] }
    }
}

function Install-PluginFrom([string]$Dir) {
    $common = @('-Yes', '-InstallDir', $Bin, '-NoModifyPath')
    switch ($Plugin) {
        'clibm-web' { $extra = @('-Port', '7878') }
        'clibm-mcp' { $extra = @('-Transport', 'stdio') }
        default { $extra = @() }
    }
    Invoke-Installer -Script (Join-Path $Dir 'install.ps1') -Arguments ($common + $extra) -Cwd $Dir
}

try {
    Write-Step "Unpacking $(Split-Path -Leaf $Zip)"
    Expand-Archive -Path $Zip -DestinationPath $Work
    $Root = Join-Path $Work $Name
    if (-not (Test-Path $Root)) { Fail "the ZIP has no top-level $Name/ folder" }

    Write-Step 'cargo build --release --locked'
    Invoke-Checked -File 'cargo' -Arguments @('build', '--release', '--locked', '--workspace') -Cwd $Root

    if ($Standalone) {
        Write-Step "Installing $Plugin with its own installer"
        Install-PluginFrom $Root
    } else {
        $profileName = if ($IsAll) { 'with-defaults' } else { 'pure' }
        Write-Step "Installing the core ($profileName)"
        Invoke-Installer -Script (Join-Path $Root 'install.ps1') -Cwd $Root -Arguments @(
            '-Profile', $profileName, '-Yes', '-Language', 'en', '-AutoCommit', 'true', '-InstallDir', $Bin, '-NoModifyPath')
        if (-not (Test-Path (Join-Path $Bin 'clibm.exe'))) { Fail "clibm.exe was not installed in $Bin" }
        if ($Plugin) {
            Write-Step "Installing $Plugin from plugins/$Folder"
            Install-PluginFrom (Join-Path $Root "plugins/$Folder")
        }
    }

    if ($Plugin -and -not (Test-Path (Join-Path $Bin "$Plugin.exe"))) { Fail "$Plugin.exe was not installed in $Bin" }

    $Clibm = Join-Path $Bin 'clibm.exe'
    if (Test-Path $Clibm) {
        Write-Step 'clibm init / status / task add / board move'
        $env:CLIBM_HOME = $HomeDir
        foreach ($a in @(@('--version'), @('init'), @('task', 'add', 'Smoke test task'), @('board', 'move', 'CLIBM-101', 'in-progress'), @('status'), @('plugin', 'list'))) {
            Invoke-Checked -File $Clibm -Arguments $a -Cwd $Project
        }
        if ($IsAll) {
            foreach ($p in 'dep-audit', 'export-md', 'workflow-lint') { Invoke-Checked -File $Clibm -Arguments @('plugin', 'run', $p) -Cwd $Project }
        }
        if ($Smoke -eq 'run') {
            Write-Step "clibm plugin run $($Plugin -replace '^clibm-', '')"
            Invoke-Checked -File $Clibm -Arguments @('plugin', 'run', ($Plugin -replace '^clibm-', '')) -Cwd $Project
        }
    }
    if ($Smoke -eq 'version') {
        Write-Step "$Plugin --version"
        Invoke-Checked -File (Join-Path $Bin "$Plugin.exe") -Arguments @('--version')
    }

    Write-Step 'Uninstalling (-Purge)'
    Invoke-Installer -Script (Join-Path $Root 'uninstall.ps1') -Arguments @('-Yes', '-Purge') -Cwd $Project -IsolatedCargo

    $left = @(Get-ChildItem -Path $Bin -Filter 'clibm*' -ErrorAction SilentlyContinue)
    if ($left.Count -gt 0) { Fail "binaries left in ${Bin}: $($left.Name -join ', ')" }
    if (-not $Standalone) {
        if (Test-Path (Join-Path $HomeDir '.clibm')) {
            $rest = @(Get-ChildItem -Recurse -File (Join-Path $HomeDir '.clibm'))
            if ($rest.Count -gt 0) { Fail "files left in $HomeDir\.clibm: $($rest.FullName -join ', ')" }
        }
        if (Test-Path (Join-Path $Project '.clibm')) { Fail "-Purge did not remove $Project\.clibm" }
    }

    Write-Host ""
    Write-Host "SMOKE OK [$Name]"
} finally {
    Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}
