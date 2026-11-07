# ==============================================================================
# CLIBM v0.1.0 — Interactive Core Installer (Windows PowerShell)
# Encoding: UTF-8 with BOM (EF BB BF) for Windows PowerShell 5.1 & 7+ compatibility
# ==============================================================================
# Presents interactive configuration options during installation:
#   1. Core Mode: Pure Core (without plugins) vs. Core + Default Plugins
#      (clibm-dep-audit, clibm-export-md, clibm-workflow-lint from .\plugins\)
#   2. Language Selection: en (English), pt-BR (Português), es (Español)
#   3. Automatic Per-Modification Commits: true (default) or false
#   4. Machine Storage / Install Location: custom path (default: ~\.clibm-bin)
#   5. User PATH Registration: add install directory to user PATH (or -NoModifyPath)

param (
    [string]$Profile = "",
    [string]$Language = "",
    [string]$AutoCommit = "",
    [string]$InstallDir = "",
    [switch]$NoModifyPath = $false,
    [switch]$Yes = $false
)

$ErrorActionPreference = "Stop"
Set-Location -Path $PSScriptRoot

function Install-ClibmEnvPathHelper {
    $HomeDir = if ($HOME) { $HOME } elseif ($env:USERPROFILE) { $env:USERPROFILE } else { [Environment]::GetFolderPath('UserProfile') }
    $SourceHelper = Join-Path $PSScriptRoot "scripts\env-path.ps1"
    $LibDir = Join-Path $HomeDir ".clibm\lib"
    $TargetHelper = Join-Path $LibDir "env-path.ps1"

    if (-not (Test-Path -LiteralPath $SourceHelper)) {
        throw "Required shared PATH helper not found at $SourceHelper"
    }

    $SourceVersion = 1
    foreach ($Line in (Get-Content -LiteralPath $SourceHelper -TotalCount 5 -Encoding UTF8)) {
        if ($Line -match 'clibm-env-path-version:\s*(\d+)') {
            $SourceVersion = [int]$Matches[1]
            break
        }
    }

    $NeedsCopy = $true
    if (Test-Path -LiteralPath $TargetHelper) {
        try {
            foreach ($Line in (Get-Content -LiteralPath $TargetHelper -TotalCount 5 -Encoding UTF8 -ErrorAction Stop)) {
                if ($Line -match 'clibm-env-path-version:\s*(\d+)') {
                    if ([int]$Matches[1] -ge $SourceVersion) {
                        $NeedsCopy = $false
                    }
                    break
                }
            }
        } catch {
            $NeedsCopy = $true
        }
    }

    if ($NeedsCopy) {
        New-Item -ItemType Directory -Force -Path $LibDir | Out-Null
        Copy-Item -LiteralPath $SourceHelper -Destination $TargetHelper -Force
    }
    return $TargetHelper
}

. (Install-ClibmEnvPathHelper)

Write-Host "===================================================================="
Write-Host " CLIBM v0.1.0 — Interactive Core Installation Wizard (UTF-8-BOM)"
Write-Host "===================================================================="

if (-not $Profile) {
    if ($Yes) {
        $Profile = "pure"
    } else {
        Write-Host ""
        Write-Host "[1/5] Select CLIBM Core Installation Profile:"
        Write-Host "  1) Pure Core (without plugins — 100% pure engine) [Default]"
        Write-Host "  2) Core + Default Plugins (builds dep-audit, export-md, workflow-lint from .\plugins\)"
        $c = Read-Host "Select option [1-2, default: 1]"
        if ($c -eq "2") { $Profile = "with-defaults" } else { $Profile = "pure" }
    }
}

if (-not $Language) {
    if ($Yes) {
        $Language = "en"
    } else {
        Write-Host ""
        Write-Host "[2/5] Select Interface & CLI Language:"
        Write-Host "  1) English (en) [Default]"
        Write-Host "  2) Português (pt-BR)"
        Write-Host "  3) Español (es)"
        $l = Read-Host "Select language [1-3, default: 1]"
        switch ($l) {
            "2" { $Language = "pt-BR" }
            "3" { $Language = "es" }
            default { $Language = "en" }
        }
    }
}

if (-not $AutoCommit) {
    if ($Yes) {
        $AutoCommit = "true"
    } else {
        Write-Host ""
        Write-Host "[3/5] Enable Automatic Per-Modification Commits (auto_commit)?"
        Write-Host "  1) Yes — Record every change individually as a BLAKE3 commit [Default]"
        Write-Host "  2) No  — Require manual 'clibm commit -m ...'"
        $a = Read-Host "Select option [1-2, default: 1]"
        if ($a -eq "2" -or $a -eq "false" -or $a -eq "no") { $AutoCommit = "false" } else { $AutoCommit = "true" }
    }
}

$DefaultBin = Join-Path $HOME ".clibm-bin"
if (-not $InstallDir) {
    if ($Yes) {
        $InstallDir = $DefaultBin
    } else {
        Write-Host ""
        $d = Read-Host "[4/5] Specify installation directory on your machine [default: $DefaultBin]"
        if ([string]::IsNullOrWhiteSpace($d)) { $InstallDir = $DefaultBin } else { $InstallDir = $d }
    }
}

$InstallDir = Resolve-ClibmNormalizedDir $InstallDir

$ModifyPath = $true
if ($NoModifyPath) {
    $ModifyPath = $false
} elseif (-not $Yes) {
    Write-Host ""
    $pAns = Read-Host "[5/5] Add installation directory ($InstallDir) to user PATH? [Y/n]"
    if ($pAns -match '^(n|no|false|2)$') {
        $ModifyPath = $false
    }
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "==> Building Pure CLIBM Core & CLI (Release Mode)..."
cargo build --release

Copy-Item "target\release\clibm.exe" -Destination (Join-Path $InstallDir "clibm.exe") -Force
Write-Host "==> Installed pure clibm.exe binary to $InstallDir\clibm.exe"

$PathStatus = "skipped"
$PathEntryAdded = $false
if ($ModifyPath) {
    $PathResult = Add-ClibmPathEntry -Owner "core" -Dir $InstallDir
    $PathStatus = $PathResult.Status
    $PathEntryAdded = [bool]$PathResult.AddedByClibm
}

Write-Host ""
Write-Host "--------------------------------------------------------------------"
Write-Host " Installation Summary (Encoding: UTF-8-BOM EF BB BF):"
Write-Host "   Profile        : $Profile"
Write-Host "   Language       : $Language"
Write-Host "   Auto-Commit    : $AutoCommit"
Write-Host "   Install Path   : $InstallDir"
Write-Host "   User PATH      : $PathStatus"
Write-Host "--------------------------------------------------------------------"

$GlobalCfgDir = Join-Path $HOME ".clibm"
$GlobalPluginsDir = Join-Path $GlobalCfgDir "plugins"
New-Item -ItemType Directory -Force -Path $GlobalPluginsDir | Out-Null

$Utf8BomEncoding = New-Object System.Text.UTF8Encoding($true)
$InstallCfg = [ordered]@{
    install_profile  = $Profile
    language         = $Language
    auto_commit      = ($AutoCommit -eq "true")
    install_path     = $InstallDir
    path_entry_added = $PathEntryAdded
    mcp_transport    = "both"
} | ConvertTo-Json -Depth 4
[System.IO.File]::WriteAllText((Join-Path $GlobalCfgDir "install-config.json"), $InstallCfg, $Utf8BomEncoding)

if ($Profile -eq "with-defaults") {
    Write-Host "==> Installing default standalone plugins from .\plugins\..."
    foreach ($PluginName in @("clibm-dep-audit", "clibm-export-md", "clibm-workflow-lint")) {
        $PluginPath = Join-Path "plugins" $PluginName
        if (Test-Path $PluginPath) {
            Push-Location $PluginPath
            if ($ModifyPath) {
                .\install.ps1 -InstallDir $InstallDir -Yes
            } else {
                .\install.ps1 -InstallDir $InstallDir -Yes -NoModifyPath
            }
            Pop-Location
        }
    }
} else {
    [System.IO.File]::WriteAllText((Join-Path $GlobalPluginsDir "registry.json"), "[]", $Utf8BomEncoding)
    Write-Host "==> Pure Core active (0 pre-installed plugins)."
}

Write-Host "===================================================================="
Write-Host " CLIBM v0.1.0 Installation Complete!"
Write-Host " Binary Location : $InstallDir\clibm.exe"
Write-Host " User PATH       : $PathStatus"
if ($ModifyPath) {
    Write-Host " Open a new terminal to use 'clibm'."
} else {
    Write-Host " Note: User PATH was not modified (-NoModifyPath)."
}
Write-Host "===================================================================="
