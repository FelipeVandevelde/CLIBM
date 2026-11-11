# ==============================================================================
# CLIBM v0.1.0 — Cross-Platform Uninstallation Script (Windows PowerShell)
# Encoding: UTF-8 with BOM (EF BB BF) for Windows PowerShell compatibility
# ==============================================================================
# Usage:
#   .\uninstall.ps1          # Interactive choice or removes binaries (preserves .clibm data)
#   .\uninstall.ps1 -Keep    # Removes clibm + plugin binaries, preserves .clibm data
#   .\uninstall.ps1 -Purge   # Removes binaries AND deletes local .clibm repository data

param (
    [switch]$Purge = $false,
    [switch]$Keep = $false,
    [switch]$Yes = $false
)

$ErrorActionPreference = "Stop"

Write-Host "============================================================"
Write-Host " CLIBM v0.1.0 Uninstaller (Windows PowerShell · UTF-8-BOM)"
Write-Host "============================================================"

if (-not $Purge -and -not $Keep -and -not $Yes) {
    try {
        Write-Host ""
        Write-Host "Select Uninstallation Mode:"
        Write-Host "  1) Standard Uninstall — Remove clibm & plugin binaries, KEEP .clibm project data [Default]"
        Write-Host "  2) Full Purge (-Purge) — Remove binaries AND delete .clibm repository data"
        $u = Read-Host "Select option [1-2, default: 1]"
        if ($u -eq "2" -or $u -eq "purge") { $Purge = $true }
    } catch {
        $Purge = $false
    }
}

$GlobalCfgDir = Join-Path $HOME ".clibm"
$InstallCfgFile = Join-Path $GlobalCfgDir "install-config.json"
$ConfiguredInstallDir = ""
$CfgPathEntryAdded = $false

if (Test-Path -LiteralPath $InstallCfgFile) {
    try {
        $RawCfg = [System.IO.File]::ReadAllText($InstallCfgFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF)
        $ParsedCfg = $RawCfg | ConvertFrom-Json
        if ($ParsedCfg.install_path) {
            $CfgDir = [string]$ParsedCfg.install_path
            if ($CfgDir -eq '~') {
                $CfgDir = $HOME
            } elseif ($CfgDir.StartsWith('~\') -or $CfgDir.StartsWith('~/')) {
                $CfgDir = Join-Path $HOME $CfgDir.Substring(2)
            }
            $ConfiguredInstallDir = $CfgDir
        }
        if ($null -ne $ParsedCfg.path_entry_added) {
            $CfgPathEntryAdded = [bool]$ParsedCfg.path_entry_added
        }
    } catch {
        # Ignore malformed install-config.json
    }
}

$HelperFile = Join-Path $GlobalCfgDir "lib\env-path.ps1"
$HelperLoaded = $false
$LedgerDirs = @()
if (Test-Path -LiteralPath $HelperFile) {
    . $HelperFile
    $HelperLoaded = $true
    $LedgerDirs = @(Get-ClibmAllLedgerDirs)
} else {
    Write-Host "-> Warning: Shared PATH helper ($HelperFile) not found; skipping user PATH ledger cleanup."
}

$RawCandidates = @()
if ($ConfiguredInstallDir) { $RawCandidates += $ConfiguredInstallDir }
if ($LedgerDirs.Count -gt 0) { $RawCandidates += $LedgerDirs }
$RawCandidates += @(
    (Join-Path $HOME ".clibm-bin"),
    (Join-Path $HOME ".local\bin"),
    $(if ($env:CARGO_HOME) { Join-Path $env:CARGO_HOME "bin" } else { Join-Path $HOME ".cargo\bin" })
)

$CandidateDirs = @()
$SeenDirs = @{}
foreach ($D in $RawCandidates) {
    if ([string]::IsNullOrWhiteSpace($D)) { continue }
    $NormKey = ($D -replace '/', '\').TrimEnd('\').ToLowerInvariant()
    if (-not $SeenDirs.ContainsKey($NormKey)) {
        $SeenDirs[$NormKey] = $true
        $CandidateDirs += $D
    }
}

$RemovedCount = 0
foreach ($Dir in $CandidateDirs) {
    if (Test-Path -LiteralPath $Dir) {
        $Binaries = Get-ChildItem -LiteralPath $Dir -Filter "clibm*.exe" -File -ErrorAction SilentlyContinue
        foreach ($Bin in $Binaries) {
            Write-Host "-> Removing binary: $($Bin.FullName)"
            Remove-Item -LiteralPath $Bin.FullName -Force
            $RemovedCount++
        }
    }
}

if ($RemovedCount -eq 0) {
    Write-Host "-> No clibm.exe or clibm-*.exe binaries found."
} else {
    Write-Host "-> Removed $RemovedCount binary(ies)."
}

$RemovedPathDirs = @()
if ($HelperLoaded) {
    $RemovedPathDirs = @(Remove-ClibmPathAll)
    if ($CfgPathEntryAdded -and $ConfiguredInstallDir) {
        $ExtraRemoved = @(Remove-ClibmDirsFromUserPathRaw -DirsToRemove @($ConfiguredInstallDir))
        if ($ExtraRemoved.Count -gt 0) {
            $RemovedPathDirs += $ExtraRemoved
        }
    }
    foreach ($PDir in $RemovedPathDirs) {
        Write-Host "-> Removed user PATH entry: $PDir"
    }
}

# Remove empty install directories created for CLIBM
$DirsToCheckEmpty = @()
if ($RemovedPathDirs.Count -gt 0) { $DirsToCheckEmpty += $RemovedPathDirs }
if ($ConfiguredInstallDir) { $DirsToCheckEmpty += $ConfiguredInstallDir }
$DirsToCheckEmpty += (Join-Path $HOME ".clibm-bin")

$CheckedEmpty = @{}
foreach ($EDir in $DirsToCheckEmpty) {
    if ([string]::IsNullOrWhiteSpace($EDir)) { continue }
    $Key = ($EDir -replace '/', '\').TrimEnd('\').ToLowerInvariant()
    if ($CheckedEmpty.ContainsKey($Key)) { continue }
    $CheckedEmpty[$Key] = $true
    if (Test-Path -LiteralPath $EDir) {
        $RemainingItems = @(Get-ChildItem -LiteralPath $EDir -Force -ErrorAction SilentlyContinue)
        if ($RemainingItems.Count -eq 0) {
            Remove-Item -LiteralPath $EDir -Force -ErrorAction SilentlyContinue
            Write-Host "-> Removed empty install directory: $EDir"
        }
    }
}

$LibDir = Join-Path $GlobalCfgDir "lib"
if (Test-Path -LiteralPath $LibDir) {
    Remove-Item -LiteralPath $LibDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "-> Removed shared helper library: $LibDir"
}

if ($Purge) {
    if (Test-Path -LiteralPath ".clibm") {
        Write-Host "-> [-Purge] Removing local .clibm\ CAS repository directory..."
        Remove-Item -LiteralPath ".clibm" -Recurse -Force
        Write-Host "-> Deleted .clibm\ repository data."
    } else {
        Write-Host "-> [-Purge] No .clibm\ directory found in current working directory."
    }
    if (Test-Path -LiteralPath $InstallCfgFile) {
        Remove-Item -LiteralPath $InstallCfgFile -Force -ErrorAction SilentlyContinue
        Write-Host "-> [-Purge] Deleted $InstallCfgFile"
    }
} else {
    Write-Host "-> Preserved local .clibm\ repository data (pass -Purge to remove data)."
}

$Utf8BomEncoding = New-Object System.Text.UTF8Encoding($true)
$Receipt = [ordered]@{
    uninstalled_binaries = $RemovedCount
    purged_data          = [bool]$Purge
    path_entries_removed = $RemovedPathDirs.Count
    encoding             = "UTF-8-BOM"
} | ConvertTo-Json
[System.IO.File]::WriteAllText((Join-Path $HOME ".clibm-uninstall-receipt.json"), $Receipt, $Utf8BomEncoding)

Write-Host "============================================================"
Write-Host " Uninstallation complete."
Write-Host "============================================================"
