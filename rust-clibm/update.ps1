# ==============================================================================
# CLIBM -- Update Script (Windows PowerShell 5.1 & 7+)
# Encoding: UTF-8 (without BOM)
# ==============================================================================
# Updates CLIBM with `clibm update`, which rebuilds it from GitHub (or from this
# clone) and replaces the installed binary.
#
# Usage:
#   iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/update.ps1)
#   .\update.ps1 [-Plugins] [-Check] [-Repo <url>] [-Branch <name>]
#
# - Inside a clone of the repository: pulls the latest changes (git pull --ff-only)
#   and builds from the clone.
# - When clibm is not installed: runs the installer instead.

$Plugins = $false
$Check = $false
$Repo = ""
$Branch = ""
for ($i = 0; $i -lt $args.Count; $i++) {
    switch -Regex ($args[$i]) {
        '^-+(?i)Plugins$' { $Plugins = $true }
        '^-+(?i)Check$'   { $Check = $true }
        '^-+(?i)Repo$'    { $Repo = $args[++$i] }
        '^-+(?i)Branch$'  { $Branch = $args[++$i] }
    }
}

$ErrorActionPreference = "Stop"
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { "" }
$HomeDir = if ($env:CLIBM_HOME) { $env:CLIBM_HOME } elseif ($HOME) { $HOME } else { $env:USERPROFILE }

function Find-ClibmBinary {
    $Cmd = Get-Command clibm -ErrorAction SilentlyContinue
    if ($Cmd) { return $Cmd.Source }
    $Cfg = Join-Path (Join-Path $HomeDir '.clibm') 'install-config.json'
    if (Test-Path -LiteralPath $Cfg) {
        try {
            $Parsed = ConvertFrom-Json -InputObject ([System.IO.File]::ReadAllText($Cfg).TrimStart([char]0xFEFF))
            if ($Parsed.install_path) {
                $Candidate = Join-Path ([string]$Parsed.install_path) 'clibm.exe'
                if (Test-Path -LiteralPath $Candidate) { return $Candidate }
            }
        } catch {}
    }
    foreach ($D in @((Join-Path $HomeDir '.clibm-bin'), (Join-Path $HomeDir '.local\bin'), (Join-Path $HomeDir '.cargo\bin'))) {
        $Candidate = Join-Path $D 'clibm.exe'
        if (Test-Path -LiteralPath $Candidate) { return $Candidate }
    }
    return $null
}

Write-Host "============================================================"
Write-Host " CLIBM Updater"
Write-Host "============================================================"

$Clibm = Find-ClibmBinary
if (-not $Clibm) {
    Write-Host "==> clibm is not installed; running the installer instead."
    $Base = if ($env:CLIBM_RAW_BASE) { $env:CLIBM_RAW_BASE.TrimEnd('/') } else { 'https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm' }
    $Installer = Join-Path ([System.IO.Path]::GetTempPath()) ("clibm-install-" + [guid]::NewGuid().ToString('N') + ".ps1")
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri "$Base/install.ps1" -OutFile $Installer -UseBasicParsing
        & $Installer -Yes
    } finally {
        Remove-Item -LiteralPath $Installer -Force -ErrorAction SilentlyContinue
    }
    return
}

# git and cargo print progress on stderr; under "Stop", Windows PowerShell 5.1 would
# abort on it when the output is redirected. Failures are detected via $LASTEXITCODE.
$ErrorActionPreference = 'Continue'

$UpdateArgs = @('update')
$InClone = $ScriptDir -and (Test-Path -LiteralPath (Join-Path $ScriptDir 'Cargo.toml')) -and (Test-Path -LiteralPath (Join-Path $ScriptDir 'src\main.rs'))
if ($InClone -and -not $Check -and -not $Repo -and -not $Branch) {
    if (Get-Command git -ErrorAction SilentlyContinue) {
        Write-Host "==> Updating the local clone (git pull --ff-only)..."
        & git -C $ScriptDir pull --ff-only
        if ($LASTEXITCODE -ne 0) { Write-Host "==> git pull failed; building the clone as it is." -ForegroundColor Yellow }
    }
    $UpdateArgs += @('--path', $ScriptDir)
} else {
    if ($Repo) { $UpdateArgs += @('--repo', $Repo) }
    if ($Branch) { $UpdateArgs += @('--branch', $Branch) }
}
if ($Check) { $UpdateArgs += '--check' }
if ($Plugins) { $UpdateArgs += '--plugins' }

& $Clibm @UpdateArgs
if ($LASTEXITCODE -ne 0) {
    Write-Host "==> Update failed (exit code $LASTEXITCODE)." -ForegroundColor Red
} else {
    Write-Host "==> Done. Verify with: clibm --version"
}
