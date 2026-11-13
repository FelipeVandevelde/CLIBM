# ==============================================================================
# clibm-plugin-template -- Uninstallation Script (Windows PowerShell)
# Encoding: UTF-8 (without BOM) for Windows PowerShell compatibility
# ==============================================================================
param (
    [switch]$Purge = $false,
    [switch]$Keep = $false,
    [switch]$Yes = $false
)

$ErrorActionPreference = "Stop"
Set-Location -Path $PSScriptRoot

# >>> clibm-env-path (generated) >>>
function Initialize-ClibmEnvPathHelper {
    try { Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction SilentlyContinue } catch {}
    $HomeDir = if ($env:CLIBM_HOME) { $env:CLIBM_HOME } elseif ($HOME) { $HOME } elseif ($env:USERPROFILE) { $env:USERPROFILE } else { [Environment]::GetFolderPath('UserProfile') }
    $LibDir = Join-Path $HomeDir '.clibm\lib'
    $HelperFile = Join-Path $LibDir 'env-path.ps1'
    $EmbeddedVersion = 6
    $EmbeddedContent = @'
# clibm-env-path-version: 6
# ==============================================================================
# CLIBM Shared User PATH Helper (Windows PowerShell 5.1 & 7+)
# Encoding: UTF-8 (without BOM)
# Manages user-scope PATH entries and reference-counted ownership in
# ~/.clibm/path-ledger without touching machine/system environment variables.
# ==============================================================================

function Get-ClibmHomeDirectory {
    if ($env:CLIBM_HOME) { return $env:CLIBM_HOME }
    if ($HOME) { return $HOME }
    if ($env:USERPROFILE) { return $env:USERPROFILE }
    return [Environment]::GetFolderPath('UserProfile')
}

function Get-ClibmRegistryEnvPath {
    if ($env:CLIBM_TEST_HKCU_ENV) { return $env:CLIBM_TEST_HKCU_ENV }
    return 'HKCU:\Environment'
}

function Resolve-ClibmNormalizedDir {
    param([Parameter(Mandatory = $true)][string]$Dir)
    $HomeDir = Get-ClibmHomeDirectory
    $Expanded = $Dir.Trim().Trim('"')
    if ($Expanded -eq '~') {
        $Expanded = $HomeDir
    } elseif ($Expanded.StartsWith('~\') -or $Expanded.StartsWith('~/')) {
        $Expanded = Join-Path $HomeDir $Expanded.Substring(2)
    }
    $Expanded = [Environment]::ExpandEnvironmentVariables($Expanded)
    if (-not [System.IO.Path]::IsPathRooted($Expanded)) {
        $BasePath = (Get-Location).ProviderPath
        $Expanded = Join-Path $BasePath $Expanded
    }
    $Full = [System.IO.Path]::GetFullPath($Expanded)
    if ($Full.Length -gt 3) {
        $Full = $Full.TrimEnd('\', '/')
    }
    return $Full
}

function ConvertTo-ClibmComparablePath {
    param([AllowEmptyString()][string]$Entry)
    if ([string]::IsNullOrWhiteSpace($Entry)) { return '' }
    $HomeDir = Get-ClibmHomeDirectory
    $Clean = $Entry.Trim().Trim('"')
    if ($Clean -eq '~') {
        $Clean = $HomeDir
    } elseif ($Clean.StartsWith('~\') -or $Clean.StartsWith('~/')) {
        $Clean = Join-Path $HomeDir $Clean.Substring(2)
    }
    $Clean = [Environment]::ExpandEnvironmentVariables($Clean)
    $Clean = $Clean -replace '/', '\'
    if ($Clean.Length -gt 3) {
        $Clean = $Clean.TrimEnd('\')
    }
    return $Clean.ToLowerInvariant()
}

function Get-ClibmRawUserPath {
    $RegPath = Get-ClibmRegistryEnvPath
    if (-not (Test-Path -LiteralPath $RegPath)) {
        return ''
    }
    $Key = Get-Item -LiteralPath $RegPath
    $Val = $Key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if ($null -eq $Val) { return '' }
    return [string]$Val
}

function Set-ClibmRawUserPath {
    param([AllowEmptyString()][string]$NewValue)
    $RegPath = Get-ClibmRegistryEnvPath
    if (-not (Test-Path -LiteralPath $RegPath)) {
        New-Item -Path $RegPath -Force | Out-Null
    }
    Set-ItemProperty -LiteralPath $RegPath -Name 'Path' -Value $NewValue -Type ExpandString
    Send-ClibmEnvChange
}

function Send-ClibmEnvChange {
    if ($env:CLIBM_TEST_HKCU_ENV) { return }
    try {
        [Environment]::SetEnvironmentVariable('CLIBM_PATH_SYNC', '1', 'User')
        [Environment]::SetEnvironmentVariable('CLIBM_PATH_SYNC', $null, 'User')
    } catch {
        # Ignore broadcast errors in restricted environments
    }
}

function Test-ClibmOnUserPath {
    param([Parameter(Mandatory = $true)][string]$Dir)
    $Target = ConvertTo-ClibmComparablePath (Resolve-ClibmNormalizedDir $Dir)
    if (-not $Target) { return $false }
    $RawUser = Get-ClibmRawUserPath
    foreach ($Seg in ($RawUser -split ';')) {
        if ([string]::IsNullOrWhiteSpace($Seg)) { continue }
        if ((ConvertTo-ClibmComparablePath $Seg) -eq $Target) {
            return $true
        }
    }
    return $false
}

function Get-ClibmLedgerPath {
    $HomeDir = Get-ClibmHomeDirectory
    return (Join-Path (Join-Path $HomeDir '.clibm') 'path-ledger')
}

function Read-ClibmLedger {
    $LedgerFile = Get-ClibmLedgerPath
    $Rows = @()
    if (-not (Test-Path -LiteralPath $LedgerFile)) {
        return $Rows
    }
    $Lines = Get-Content -LiteralPath $LedgerFile -Encoding UTF8 -ErrorAction SilentlyContinue
    foreach ($RawLine in $Lines) {
        if ($null -eq $RawLine) { continue }
        $Line = $RawLine.TrimStart([char]0xFEFF).TrimEnd("`r", "`n")
        if ([string]::IsNullOrWhiteSpace($Line)) { continue }
        $Parts = $Line -split "`t", 3
        if ($Parts.Count -lt 3) { continue }
        $Rows += [PSCustomObject]@{
            Owner = $Parts[0].Trim()
            Dir   = $Parts[1].Trim()
            Added = if ($Parts[2].Trim() -eq '1') { 1 } else { 0 }
        }
    }
    return $Rows
}

function Write-ClibmLedger {
    param([AllowEmptyCollection()][array]$Rows)
    $LedgerFile = Get-ClibmLedgerPath
    if (-not $Rows -or $Rows.Count -eq 0) {
        if (Test-Path -LiteralPath $LedgerFile) {
            Remove-Item -LiteralPath $LedgerFile -Force -ErrorAction SilentlyContinue
        }
        return
    }
    $Parent = Split-Path -Parent $LedgerFile
    if (-not (Test-Path -LiteralPath $Parent)) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $OutLines = foreach ($R in $Rows) {
        "$($R.Owner)`t$($R.Dir)`t$($R.Added)"
    }
    $Content = ($OutLines -join "`n") + "`n"
    $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($LedgerFile, $Content, $Utf8NoBom)
}

function Add-ClibmDirToUserPathRaw {
    param([Parameter(Mandatory = $true)][string]$NormDir)
    $Target = ConvertTo-ClibmComparablePath $NormDir
    $RawUser = Get-ClibmRawUserPath
    $Segments = @($RawUser -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $Already = $false
    foreach ($S in $Segments) {
        if ((ConvertTo-ClibmComparablePath $S) -eq $Target) {
            $Already = $true
            break
        }
    }
    if (-not $Already) {
        $Segments += $NormDir
        Set-ClibmRawUserPath ($Segments -join ';')
    }
    $SessionSegs = @($env:Path -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $InSession = $false
    foreach ($S in $SessionSegs) {
        if ((ConvertTo-ClibmComparablePath $S) -eq $Target) {
            $InSession = $true
            break
        }
    }
    if (-not $InSession) {
        if ([string]::IsNullOrWhiteSpace($env:Path)) {
            $env:Path = $NormDir
        } else {
            $env:Path = "$($env:Path.TrimEnd(';'));$NormDir"
        }
    }
}

function Remove-ClibmDirsFromUserPathRaw {
    param([AllowEmptyCollection()][string[]]$DirsToRemove)
    if (-not $DirsToRemove -or $DirsToRemove.Count -eq 0) { return @() }
    $TargetSet = @{}
    foreach ($D in $DirsToRemove) {
        $Comp = ConvertTo-ClibmComparablePath $D
        if ($Comp) { $TargetSet[$Comp] = $D }
    }
    if ($TargetSet.Count -eq 0) { return @() }

    $RawUser = Get-ClibmRawUserPath
    $Segments = @($RawUser -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $Kept = @()
    $ActuallyRemoved = @()
    $RemovedSeen = @{}
    foreach ($S in $Segments) {
        $Comp = ConvertTo-ClibmComparablePath $S
        if ($TargetSet.ContainsKey($Comp)) {
            if (-not $RemovedSeen.ContainsKey($Comp)) {
                $RemovedSeen[$Comp] = $true
                $ActuallyRemoved += $TargetSet[$Comp]
            }
        } else {
            $Kept += $S
        }
    }
    if ($ActuallyRemoved.Count -gt 0) {
        Set-ClibmRawUserPath ($Kept -join ';')
    }

    $SessionKept = @()
    foreach ($S in ($env:Path -split ';')) {
        if ([string]::IsNullOrWhiteSpace($S)) { continue }
        $Comp = ConvertTo-ClibmComparablePath $S
        if (-not $TargetSet.ContainsKey($Comp)) {
            $SessionKept += $S
        }
    }
    $env:Path = ($SessionKept -join ';')
    return $ActuallyRemoved
}

function Add-ClibmPathEntry {
    param(
        [Parameter(Mandatory = $true)][string]$Owner,
        [Parameter(Mandatory = $true)][string]$Dir
    )
    $OwnerClean = $Owner.Trim()
    $NormDir = Resolve-ClibmNormalizedDir $Dir
    $CompDir = ConvertTo-ClibmComparablePath $NormDir
    $Rows = @(Read-ClibmLedger)

    $OnUserPath = Test-ClibmOnUserPath -Dir $NormDir
    $ExistingAddedFlag = 0
    foreach ($R in $Rows) {
        if ($null -eq $R) { continue }
        if ((ConvertTo-ClibmComparablePath $R.Dir) -eq $CompDir -and $R.Added -eq 1) {
            $ExistingAddedFlag = 1
            break
        }
    }

    if (-not $OnUserPath) {
        Add-ClibmDirToUserPathRaw -NormDir $NormDir
        $AddedByClibm = 1
        $Status = 'added'
    } else {
        $SessionHas = $false
        foreach ($S in ($env:Path -split ';')) {
            if ((ConvertTo-ClibmComparablePath $S) -eq $CompDir) { $SessionHas = $true; break }
        }
        if (-not $SessionHas) {
            $env:Path = if ($env:Path) { "$($env:Path.TrimEnd(';'));$NormDir" } else { $NormDir }
        }
        $AddedByClibm = $ExistingAddedFlag
        $Status = 'already present'
    }

    $OldOwnerDirs = @()
    $UpdatedRows = @()
    foreach ($R in $Rows) {
        if ($null -eq $R) { continue }
        if ($R.Owner -ieq $OwnerClean) {
            if ((ConvertTo-ClibmComparablePath $R.Dir) -ne $CompDir) {
                $OldOwnerDirs += $R
            }
        } else {
            if ((ConvertTo-ClibmComparablePath $R.Dir) -eq $CompDir -and $AddedByClibm -eq 1) {
                $R.Added = 1
            }
            $UpdatedRows += $R
        }
    }
    $UpdatedRows += [PSCustomObject]@{
        Owner = $OwnerClean
        Dir   = $NormDir
        Added = $AddedByClibm
    }

    $OrphanDirs = @()
    foreach ($Old in $OldOwnerDirs) {
        if ($Old.Added -ne 1) { continue }
        $OldComp = ConvertTo-ClibmComparablePath $Old.Dir
        $StillOwned = $false
        foreach ($U in $UpdatedRows) {
            if ((ConvertTo-ClibmComparablePath $U.Dir) -eq $OldComp) {
                $StillOwned = $true
                break
            }
        }
        if (-not $StillOwned) {
            $OrphanDirs += $Old.Dir
        }
    }
    if ($OrphanDirs.Count -gt 0) {
        Remove-ClibmDirsFromUserPathRaw -DirsToRemove $OrphanDirs | Out-Null
    }

    Write-ClibmLedger -Rows $UpdatedRows
    return [PSCustomObject]@{
        Status       = $Status
        Dir          = $NormDir
        AddedByClibm = [bool]($AddedByClibm -eq 1)
    }
}

function Get-ClibmOwnerDirs {
    param([Parameter(Mandatory = $true)][string]$Owner)
    $OwnerClean = $Owner.Trim()
    $Rows = @(Read-ClibmLedger)
    $Dirs = @()
    foreach ($R in $Rows) {
        if ($null -ne $R -and ($R.Owner -ieq $OwnerClean)) {
            $Dirs += $R.Dir
        }
    }
    return $Dirs
}

function Get-ClibmAllLedgerDirs {
    $Rows = @(Read-ClibmLedger)
    $Seen = @{}
    $Dirs = @()
    foreach ($R in $Rows) {
        if ($null -eq $R) { continue }
        $Comp = ConvertTo-ClibmComparablePath $R.Dir
        if ($Comp -and -not $Seen.ContainsKey($Comp)) {
            $Seen[$Comp] = $true
            $Dirs += $R.Dir
        }
    }
    return $Dirs
}

function Remove-ClibmPathEntry {
    param([Parameter(Mandatory = $true)][string]$Owner)
    $OwnerClean = $Owner.Trim()
    $Rows = @(Read-ClibmLedger)
    if ($Rows.Count -eq 0) { return @() }

    $OwnerRows = @()
    $Remaining = @()
    foreach ($R in $Rows) {
        if ($null -eq $R) { continue }
        if ($R.Owner -ieq $OwnerClean) {
            $OwnerRows += $R
        } else {
            $Remaining += $R
        }
    }
    if ($OwnerRows.Count -eq 0) { return @() }

    $CandidateRemove = @()
    $Seen = @{}
    foreach ($ORow in $OwnerRows) {
        if ($ORow.Added -ne 1) { continue }
        $Comp = ConvertTo-ClibmComparablePath $ORow.Dir
        if ($Seen.ContainsKey($Comp)) { continue }
        $Seen[$Comp] = $true
        $StillReferenced = $false
        foreach ($Rem in $Remaining) {
            if ((ConvertTo-ClibmComparablePath $Rem.Dir) -eq $Comp) {
                $StillReferenced = $true
                break
            }
        }
        if (-not $StillReferenced) {
            $CandidateRemove += $ORow.Dir
        }
    }

    $Removed = @()
    if ($CandidateRemove.Count -gt 0) {
        $Removed = @(Remove-ClibmDirsFromUserPathRaw -DirsToRemove $CandidateRemove)
    }
    Write-ClibmLedger -Rows $Remaining
    return $Removed
}

function Remove-ClibmPathAll {
    $Rows = @(Read-ClibmLedger)
    if ($Rows.Count -eq 0) { return @() }
    $CandidateRemove = @()
    $Seen = @{}
    foreach ($R in $Rows) {
        if ($null -eq $R -or $R.Added -ne 1) { continue }
        $Comp = ConvertTo-ClibmComparablePath $R.Dir
        if ($Comp -and -not $Seen.ContainsKey($Comp)) {
            $Seen[$Comp] = $true
            $CandidateRemove += $R.Dir
        }
    }
    $Removed = @()
    if ($CandidateRemove.Count -gt 0) {
        $Removed = @(Remove-ClibmDirsFromUserPathRaw -DirsToRemove $CandidateRemove)
    }
    Write-ClibmLedger -Rows @()
    return $Removed
}

function Test-ClibmCoreInstalled {
    param([string]$ExtraDir = '')
    $HomeDir = Get-ClibmHomeDirectory
    $InstallCfg = Join-Path (Join-Path $HomeDir '.clibm') 'install-config.json'
    if (Test-Path -LiteralPath $InstallCfg) { return $true }
    if (Get-Command 'clibm' -ErrorAction SilentlyContinue) { return $true }
    $CheckDirs = @(
        (Join-Path $HomeDir '.clibm-bin'),
        (Join-Path $HomeDir '.local\bin'),
        (Join-Path $HomeDir '.cargo\bin')
    )
    if ($ExtraDir) { $CheckDirs = @($ExtraDir) + $CheckDirs }
    foreach ($D in $CheckDirs) {
        if ($D -and (Test-Path -LiteralPath (Join-Path $D 'clibm.exe'))) {
            return $true
        }
    }
    return $false
}

function Get-ClibmRawRepoBase {
    if ($env:CLIBM_RAW_BASE) {
        return $env:CLIBM_RAW_BASE.TrimEnd('/')
    }
    return 'https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm'
}

function Resolve-ClibmLocalSourceDir {
    param(
        [Parameter(Mandatory = $true)][string]$CrateName,
        [string]$RemoteSubPath = '',
        [string]$CandidateDir = ''
    )
    $SearchDirs = @()
    if (-not [string]::IsNullOrWhiteSpace($CandidateDir)) {
        $SearchDirs += $CandidateDir
        if (-not [string]::IsNullOrWhiteSpace($RemoteSubPath)) {
            $SearchDirs += (Join-Path $CandidateDir ($RemoteSubPath -replace '/', '\'))
            $SearchDirs += (Join-Path (Join-Path $CandidateDir 'rust-clibm') ($RemoteSubPath -replace '/', '\'))
        } else {
            $SearchDirs += (Join-Path $CandidateDir 'rust-clibm')
        }
    }
    $Cwd = (Get-Location).ProviderPath
    if ($Cwd -and ($SearchDirs -notcontains $Cwd)) {
        $SearchDirs += $Cwd
        if (-not [string]::IsNullOrWhiteSpace($RemoteSubPath)) {
            $SearchDirs += (Join-Path $Cwd ($RemoteSubPath -replace '/', '\'))
            $SearchDirs += (Join-Path (Join-Path $Cwd 'rust-clibm') ($RemoteSubPath -replace '/', '\'))
        } else {
            $SearchDirs += (Join-Path $Cwd 'rust-clibm')
        }
    }
    foreach ($Dir in $SearchDirs) {
        if ([string]::IsNullOrWhiteSpace($Dir)) { continue }
        $CargoFile = Join-Path $Dir 'Cargo.toml'
        $MainFile  = Join-Path $Dir 'src\main.rs'
        if ((Test-Path -LiteralPath $CargoFile) -and (Test-Path -LiteralPath $MainFile)) {
            try {
                $Toml = Get-Content -LiteralPath $CargoFile -Raw -Encoding UTF8 -ErrorAction Stop
                $Escaped = [regex]::Escape($CrateName)
                if ($Toml -match ('(?m)^\s*name\s*=\s*"' + $Escaped + '"')) {
                    return $Dir
                }
            } catch {
                # Continue searching
            }
        }
    }
    return ''
}

function Get-ClibmGitSource {
    $Url = if ($env:CLIBM_GIT_URL) { $env:CLIBM_GIT_URL } else { 'https://github.com/FelipeVandevelde/CLIBM.git' }
    $Ref = if ($env:CLIBM_GIT_REF) { $env:CLIBM_GIT_REF } else { 'main' }
    return [PSCustomObject]@{ Url = $Url; Ref = $Ref }
}

# Copies a freshly built binary over the installed one. A binary that Windows keeps
# locked (it is running) is renamed out of the way first; leftovers from earlier
# renames are deleted when possible.
function Copy-ClibmBinary {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )
    $DestDir = Split-Path -Parent $Destination
    if ($DestDir -and -not (Test-Path -LiteralPath $DestDir)) {
        New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
    }
    if (Test-Path -LiteralPath $Destination) {
        try {
            Remove-Item -LiteralPath $Destination -Force -ErrorAction Stop
        } catch {
            $Rotated = "$Destination.old." + [guid]::NewGuid().ToString('N').Substring(0, 8)
            Move-Item -LiteralPath $Destination -Destination $Rotated -Force -ErrorAction Stop
        }
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
    $Leaf = Split-Path -Leaf $Destination
    Get-ChildItem -LiteralPath $DestDir -Filter "$Leaf.old.*" -ErrorAction SilentlyContinue | ForEach-Object {
        Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue
    }
}

# Builds a CLIBM crate and installs its binary at DestinationBinary.
# - Local mode (a clone of the repository is found): `cargo build --release -p <crate>`.
# - Remote mode: `cargo install --git <CLIBM_GIT_URL> --branch <CLIBM_GIT_REF> --root <temp>`,
#   which builds the whole Cargo workspace correctly. Set CLIBM_FORCE_REMOTE=1 to skip
#   local mode. SourceFiles and RawBaseUrl are accepted for older callers but unused.
# Sets $script:ClibmLastBuildCommit to the commit that was built (when known).
function Invoke-ClibmBuildAndStage {
    param(
        [Parameter(Mandatory = $true)][string]$CrateName,
        [AllowEmptyString()][string]$RemoteSubPath = '',
        [AllowEmptyCollection()][string[]]$SourceFiles = @(),
        [Parameter(Mandatory = $true)][string]$DestinationBinary,
        [string]$LocalCandidateDir = '',
        [string]$RawBaseUrl = '',
        [AllowEmptyCollection()][string[]]$CargoFeatures = @()
    )
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo was not found. Install Rust from https://rustup.rs, open a new terminal and run the installer again."
    }
    $script:ClibmLastBuildCommit = ''

    $LocalDir = ''
    if (-not $env:CLIBM_FORCE_REMOTE) {
        $LocalDir = Resolve-ClibmLocalSourceDir -CrateName $CrateName -RemoteSubPath $RemoteSubPath -CandidateDir $LocalCandidateDir
    }

    if (-not [string]::IsNullOrWhiteSpace($LocalDir)) {
        Write-Host "==> [Local Mode] Building $CrateName from local source ($LocalDir)..."
        Push-Location -LiteralPath $LocalDir
        try {
            $TargetDir = ''
            try {
                $Meta = (& cargo metadata --no-deps --format-version 1) | Out-String | ConvertFrom-Json
                $TargetDir = [string]$Meta.target_directory
            } catch {}
            if (-not $TargetDir) { $TargetDir = Join-Path $LocalDir 'target' }
            $BuiltBin = Join-Path $TargetDir "release\$CrateName.exe"
            if (Test-Path -LiteralPath $BuiltBin) {
                # A running copy (e.g. started from target\release) would block the linker.
                try { Remove-Item -LiteralPath $BuiltBin -Force -ErrorAction Stop } catch {
                    try { Move-Item -LiteralPath $BuiltBin -Destination ("$BuiltBin.old." + [guid]::NewGuid().ToString('N').Substring(0, 8)) -Force -ErrorAction Stop } catch {}
                }
            }
            $CargoArgs = @('build', '--release', '-p', $CrateName) + $CargoFeatures
            & cargo @CargoArgs | Out-Host
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build --release failed for $CrateName (exit code $LASTEXITCODE)"
            }
            if (-not (Test-Path -LiteralPath $BuiltBin)) {
                throw "Compiled binary not found at $BuiltBin"
            }
            Copy-ClibmBinary -Source $BuiltBin -Destination $DestinationBinary
            if (Get-Command git -ErrorAction SilentlyContinue) {
                $Sha = (& git -C $LocalDir rev-parse HEAD 2>$null | Out-String).Trim()
                if ($Sha -match '^[0-9a-f]{40}$') { $script:ClibmLastBuildCommit = $Sha }
            }
        } finally {
            Pop-Location
        }
        return 'local'
    }

    $Src = Get-ClibmGitSource
    $TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("$CrateName-build-" + [guid]::NewGuid().ToString('N'))
    Write-Host "==> [Remote Mode] Building $CrateName from $($Src.Url) (branch $($Src.Ref))..."
    try {
        $CargoArgs = @('install', '--git', $Src.Url, '--branch', $Src.Ref, '--root', $TempRoot, '--locked', '--force', $CrateName) + $CargoFeatures
        # With <temp>in on PATH, cargo does not ask the user to add the temp folder to PATH.
        $SavedPath = $env:PATH
        $env:PATH = (Join-Path $TempRoot 'bin') + [System.IO.Path]::PathSeparator + $env:PATH
        try { & cargo @CargoArgs | Out-Host } finally { $env:PATH = $SavedPath }
        if ($LASTEXITCODE -ne 0) {
            throw "cargo install failed for $CrateName (exit code $LASTEXITCODE)"
        }
        $BuiltBin = Join-Path $TempRoot "bin\$CrateName.exe"
        if (-not (Test-Path -LiteralPath $BuiltBin)) {
            throw "Compiled binary not found at $BuiltBin"
        }
        Copy-ClibmBinary -Source $BuiltBin -Destination $DestinationBinary
        $CratesJson = Join-Path $TempRoot '.crates2.json'
        if (Test-Path -LiteralPath $CratesJson) {
            $Raw = [System.IO.File]::ReadAllText($CratesJson)
            if ($Raw -match '#([0-9a-f]{40})') { $script:ClibmLastBuildCommit = $Matches[1] }
        }
    } finally {
        if (Test-Path -LiteralPath $TempRoot) {
            Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    return 'remote'
}

function Find-ClibmProjectPluginDirs {
    param([AllowEmptyCollection()][string[]]$SearchRoots = @())
    $HomeDir = Get-ClibmHomeDirectory
    $GlobalClibmSet = @{}
    foreach ($H in @($HomeDir, $HOME, $env:USERPROFILE)) {
        if (-not [string]::IsNullOrWhiteSpace($H)) {
            $C = ConvertTo-ClibmComparablePath (Join-Path $H '.clibm')
            if ($C) { $GlobalClibmSet[$C] = $true }
        }
    }
    $Seen = @{}
    $Result = @()
    $Candidates = @()
    if ($env:CLIBM_PROJECT_ROOT) { $Candidates += $env:CLIBM_PROJECT_ROOT }
    foreach ($R in $SearchRoots) {
        if (-not [string]::IsNullOrWhiteSpace($R)) { $Candidates += $R }
    }
    $Cwd = (Get-Location).ProviderPath
    if ($Cwd) { $Candidates += $Cwd }

    foreach ($Root in $Candidates) {
        if ([string]::IsNullOrWhiteSpace($Root)) { continue }
        try {
            $Curr = Resolve-ClibmNormalizedDir $Root
        } catch {
            continue
        }
        $StartCurr = $Curr
        while (-not [string]::IsNullOrWhiteSpace($Curr)) {
            $Leaf = Split-Path -Leaf $Curr
            $Parent = Split-Path -Parent $Curr
            if ($Leaf -ieq 'plugins' -and $Parent -and ((Split-Path -Leaf $Parent) -ieq '.clibm')) {
                $ClibmComp = ConvertTo-ClibmComparablePath $Parent
                if ($ClibmComp -and -not $GlobalClibmSet.ContainsKey($ClibmComp) -and -not $Seen.ContainsKey($ClibmComp)) {
                    $Seen[$ClibmComp] = $true
                    $Result += $Curr
                }
            } elseif ($Leaf -ieq '.clibm') {
                $ClibmComp = ConvertTo-ClibmComparablePath $Curr
                if ($ClibmComp -and -not $GlobalClibmSet.ContainsKey($ClibmComp) -and -not $Seen.ContainsKey($ClibmComp)) {
                    $Seen[$ClibmComp] = $true
                    $Result += (Join-Path $Curr 'plugins')
                }
            } else {
                $DotClibm = Join-Path $Curr '.clibm'
                $ClibmComp = ConvertTo-ClibmComparablePath $DotClibm
                if ($ClibmComp -and -not $GlobalClibmSet.ContainsKey($ClibmComp) -and -not $Seen.ContainsKey($ClibmComp)) {
                    $HasDotClibm = try { Test-Path -LiteralPath $DotClibm -ErrorAction SilentlyContinue } catch { $false }
                    if ($HasDotClibm) {
                        $IsRepo = ($Curr -eq $StartCurr) -or
                            (try { Test-Path -LiteralPath (Join-Path $DotClibm 'objects') -ErrorAction SilentlyContinue } catch { $false }) -or
                            (try { Test-Path -LiteralPath (Join-Path $DotClibm 'HEAD') -ErrorAction SilentlyContinue } catch { $false }) -or
                            (try { Test-Path -LiteralPath (Join-Path $DotClibm 'config.json') -ErrorAction SilentlyContinue } catch { $false })
                        if ($IsRepo) {
                            $Seen[$ClibmComp] = $true
                            $Result += (Join-Path $DotClibm 'plugins')
                        }
                    }
                }
            }
            if (-not $Parent -or ($Parent -eq $Curr)) { break }
            $Curr = $Parent
        }
    }
    return $Result
}

function Get-ClibmGlobalRegistryFile {
    $HomeDir = Get-ClibmHomeDirectory
    return (Join-Path (Join-Path (Join-Path $HomeDir '.clibm') 'plugins') 'registry.json')
}

# Registry files are JSON arrays. The global registry (~/.clibm/plugins/registry.json)
# holds one manifest per installed plugin; a project's .clibm/plugins/registry.json only
# holds {"name": ..., "enabled": ...} overrides. Files are written with one compact
# object per line, UTF-8 without BOM.
function Read-ClibmRegistryEntries {
    param([Parameter(Mandatory = $true)][string]$RegFile)
    if (-not (Test-Path -LiteralPath $RegFile)) { return @() }
    try {
        $Raw = [System.IO.File]::ReadAllText($RegFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF).Trim()
        if (-not $Raw -or $Raw -eq '[]') { return @() }
        $Parsed = ConvertFrom-Json -InputObject $Raw
        return @($Parsed | Where-Object { $null -ne $_ -and -not [string]::IsNullOrWhiteSpace([string]$_.name) })
    } catch {
        return @()
    }
}

function Write-ClibmRegistryEntries {
    param(
        [Parameter(Mandatory = $true)][string]$RegFile,
        [AllowEmptyCollection()][object[]]$Entries = @()
    )
    $Parent = Split-Path -Parent $RegFile
    if ($Parent -and -not (Test-Path -LiteralPath $Parent)) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $Lines = @()
    foreach ($E in @($Entries)) {
        if ($null -eq $E) { continue }
        $Lines += (ConvertTo-Json -InputObject $E -Depth 20 -Compress)
    }
    if ($Lines.Count -eq 0) {
        $Body = "[]`n"
    } else {
        $Body = "[`n" + ($Lines -join ",`n") + "`n]`n"
    }
    $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($RegFile, $Body, $Utf8NoBom)
}

# Asks a plugin binary for its registry entry (`<exe> --manifest --executable <exe>`).
function Get-ClibmPluginManifest {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$HooksCsv = ''
    )
    if (-not (Test-Path -LiteralPath $Executable)) { return $null }
    try {
        $ArgList = @('--manifest', '--executable', ($Executable -replace '\\', '/'))
        if (-not [string]::IsNullOrWhiteSpace($HooksCsv)) { $ArgList += @('--hooks', $HooksCsv) }
        $Out = & $Executable @ArgList
        if ($LASTEXITCODE -ne 0) { return $null }
        $Line = @($Out | Where-Object { ([string]$_).Trim().StartsWith('{') }) | Select-Object -First 1
        if (-not $Line) { return $null }
        $Manifest = ConvertFrom-Json -InputObject ([string]$Line)
        if ($null -eq $Manifest -or [string]::IsNullOrWhiteSpace([string]$Manifest.name)) { return $null }
        return $Manifest
    } catch {
        return $null
    }
}

function Set-ClibmProjectPluginOverride {
    param(
        [Parameter(Mandatory = $true)][string]$PluginsDir,
        [Parameter(Mandatory = $true)][string]$Name,
        [bool]$Enabled = $true,
        [switch]$Remove
    )
    $RegFile = Join-Path $PluginsDir 'registry.json'
    $Overrides = @()
    foreach ($E in @(Read-ClibmRegistryEntries -RegFile $RegFile)) {
        if ($E.name -eq $Name) { continue }
        $En = $true
        if ($null -ne $E.enabled) { $En = [bool]$E.enabled }
        $Overrides += [PSCustomObject][ordered]@{ name = [string]$E.name; enabled = $En }
    }
    if (-not $Remove) {
        $Overrides += [PSCustomObject][ordered]@{ name = $Name; enabled = $Enabled }
    }
    if ((-not $Remove) -or (Test-Path -LiteralPath $RegFile)) {
        Write-ClibmRegistryEntries -RegFile $RegFile -Entries $Overrides
    }
}

# Registers an installed plugin in the global registry. The entry comes from the plugin
# itself (`--manifest`), so MCP tool declarations are kept; plugins without `--manifest`
# get a basic entry. Scope 'project-only' installs the plugin disabled by default and
# enables it only for the current project.
function Register-ClibmGlobalPlugin {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Description,
        [Parameter(Mandatory = $true)][string[]]$Hooks,
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$Version = '0.1.0',
        [string]$Scope = 'global',
        [string]$ProjectDir = '',
        [string]$InstallDir = ''
    )
    $CleanHooks = @()
    foreach ($RawH in $Hooks) {
        foreach ($Part in ($RawH -split ',')) {
            $Trimmed = $Part.Trim()
            if (-not [string]::IsNullOrWhiteSpace($Trimmed)) {
                $CleanHooks += $Trimmed
            }
        }
    }
    if ($CleanHooks.Count -eq 0) { $CleanHooks = @('manual-run') }

    $ExePath = ($Executable -replace '\\', '/')
    $Entry = Get-ClibmPluginManifest -Executable $Executable -HooksCsv ($CleanHooks -join ',')
    if ($null -eq $Entry) {
        $Entry = [PSCustomObject][ordered]@{
            name        = $Name
            version     = $Version
            description = $Description
            enabled     = $true
            hooks       = [string[]]$CleanHooks
            executable  = $ExePath
        }
    }
    $Entry.executable = $ExePath
    $Entry.enabled = ($Scope -ne 'project-only')

    $GlobalRegFile = Get-ClibmGlobalRegistryFile
    $Entries = @(Read-ClibmRegistryEntries -RegFile $GlobalRegFile | Where-Object { $_.name -ne $Name })
    $Entries += $Entry
    $Entries = @($Entries | Sort-Object -Property name)
    Write-ClibmRegistryEntries -RegFile $GlobalRegFile -Entries $Entries
    Write-Host "==> Registered $Name in plugin registry: $GlobalRegFile"

    if ($Scope -eq 'project-only') {
        $BaseProj = if (-not [string]::IsNullOrWhiteSpace($ProjectDir)) { $ProjectDir } else { (Get-Location).ProviderPath }
        $PDir = @(Find-ClibmProjectPluginDirs -SearchRoots @($BaseProj)) | Select-Object -First 1
        if (-not $PDir) { $PDir = Join-Path (Join-Path $BaseProj '.clibm') 'plugins' }
        Set-ClibmProjectPluginOverride -PluginsDir $PDir -Name $Name -Enabled $true
        Write-Host "==> Enabled $Name for this project only: $(Join-Path $PDir 'registry.json')"
    }
}

# Removes a plugin from the global registry, from the overrides of the projects found
# around ProjectDir / ExtraDirs, and from legacy registry.json files next to binaries.
function Unregister-ClibmPlugin {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$ProjectDir = '',
        [string[]]$ExtraDirs = @()
    )
    $Files = @(Get-ClibmGlobalRegistryFile)
    foreach ($EDir in $ExtraDirs) {
        if (-not [string]::IsNullOrWhiteSpace($EDir)) { $Files += (Join-Path $EDir 'registry.json') }
    }
    $Seen = @{}
    foreach ($RegFile in $Files) {
        $Comp = ConvertTo-ClibmComparablePath $RegFile
        if (-not $Comp -or $Seen.ContainsKey($Comp)) { continue }
        $Seen[$Comp] = $true
        $Before = @(Read-ClibmRegistryEntries -RegFile $RegFile)
        $After = @($Before | Where-Object { $_.name -ne $Name })
        if ($After.Count -ne $Before.Count) {
            Write-ClibmRegistryEntries -RegFile $RegFile -Entries $After
            Write-Host "==> Removed $Name from plugin registry: $RegFile"
        }
    }
    foreach ($PDir in @(Find-ClibmProjectPluginDirs -SearchRoots (@($ProjectDir) + $ExtraDirs))) {
        $RegFile = Join-Path $PDir 'registry.json'
        $Had = @(Read-ClibmRegistryEntries -RegFile $RegFile | Where-Object { $_.name -eq $Name }).Count -gt 0
        if ($Had) {
            Set-ClibmProjectPluginOverride -PluginsDir $PDir -Name $Name -Remove
            Write-Host "==> Removed $Name from project plugin settings: $RegFile"
        }
    }
}

# Uninstalls a plugin: stops running copies, deletes its binary from every known install
# folder (renaming it when Windows keeps it locked), unregisters it and releases its
# user PATH ownership. Never stops on a single failure.
function Uninstall-ClibmPlugin {
    param([Parameter(Mandatory = $true)][string]$Name)
    $ExeName = "$Name.exe"
    $HomeDir = Get-ClibmHomeDirectory

    $Candidates = @(Get-ClibmOwnerDirs -Owner $Name)
    foreach ($E in @(Read-ClibmRegistryEntries -RegFile (Get-ClibmGlobalRegistryFile))) {
        if ($E.name -eq $Name -and $E.executable) {
            $Candidates += (Split-Path -Parent ([string]$E.executable -replace '/', '\'))
        }
    }
    $InstallCfg = Join-Path (Join-Path $HomeDir '.clibm') 'install-config.json'
    if (Test-Path -LiteralPath $InstallCfg) {
        try {
            $Cfg = ConvertFrom-Json -InputObject ([System.IO.File]::ReadAllText($InstallCfg, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF))
            if ($Cfg.install_path) { $Candidates += (Resolve-ClibmNormalizedDir ([string]$Cfg.install_path)) }
        } catch {}
    }
    $Candidates += @((Join-Path $HomeDir '.clibm-bin'), (Join-Path $HomeDir '.local\bin'), (Join-Path $HomeDir '.cargo\bin'))

    $SearchDirs = @()
    $Seen = @{}
    foreach ($D in $Candidates) {
        if ([string]::IsNullOrWhiteSpace($D)) { continue }
        $K = ConvertTo-ClibmComparablePath $D
        if ($K -and -not $Seen.ContainsKey($K)) { $Seen[$K] = $true; $SearchDirs += $D }
    }

    $Running = @(Get-Process -Name $Name -ErrorAction SilentlyContinue)
    if ($Running.Count -gt 0) {
        Write-Host "==> Stopping $($Running.Count) running $Name process(es)..."
        $Running | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 400
    }

    $Failed = $false
    foreach ($Dir in $SearchDirs) {
        if (-not (Test-Path -LiteralPath $Dir)) { continue }
        $Bin = Join-Path $Dir $ExeName
        if (Test-Path -LiteralPath $Bin) {
            try {
                Remove-Item -LiteralPath $Bin -Force -ErrorAction Stop
                Write-Host "==> Removed binary: $Bin"
            } catch {
                $Rotated = "$Bin.old." + [guid]::NewGuid().ToString('N').Substring(0, 8)
                try {
                    Move-Item -LiteralPath $Bin -Destination $Rotated -Force -ErrorAction Stop
                    Write-Host "==> Binary was locked; renamed to $Rotated (deleted on the next install/uninstall)"
                } catch {
                    $Failed = $true
                    Write-Host "==> WARNING: could not remove $Bin. Close programs using it and run the uninstaller again." -ForegroundColor Yellow
                }
            }
        }
        Get-ChildItem -LiteralPath $Dir -Filter "$ExeName.old.*" -ErrorAction SilentlyContinue | ForEach-Object {
            Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue
        }
    }

    Unregister-ClibmPlugin -Name $Name -ProjectDir (Get-Location).ProviderPath -ExtraDirs $SearchDirs
    $RemovedPathDirs = @(Remove-ClibmPathEntry -Owner $Name)
    foreach ($RDir in $RemovedPathDirs) {
        Write-Host "==> Removed user PATH entry: $RDir"
        if ((Test-Path -LiteralPath $RDir) -and @(Get-ChildItem -LiteralPath $RDir -Force -ErrorAction SilentlyContinue).Count -eq 0) {
            Remove-Item -LiteralPath $RDir -Force -ErrorAction SilentlyContinue
        }
    }
    return (-not $Failed)
}
'@
    $script:ClibmEmbeddedHelperContent = $EmbeddedContent
    $NeedsWrite = $true
    if (Test-Path -LiteralPath $HelperFile) {
        try {
            $HeadLines = Get-Content -LiteralPath $HelperFile -TotalCount 5 -Encoding UTF8 -ErrorAction Stop
            foreach ($Line in $HeadLines) {
                if ($Line -match 'clibm-env-path-version:\s*(\d+)') {
                    if ([int]$Matches[1] -gt $EmbeddedVersion) {
                        $NeedsWrite = $false
                    } elseif ([int]$Matches[1] -eq $EmbeddedVersion) {
                        $ExistingClean = ([System.IO.File]::ReadAllText($HelperFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF) -replace "`r`n", "`n").TrimEnd("`n")
                        $TargetClean = ($EmbeddedContent -replace "`r`n", "`n").TrimEnd("`n")
                        if ($ExistingClean -ceq $TargetClean) {
                            $NeedsWrite = $false
                        }
                    }
                    break
                }
            }
        } catch {
            $NeedsWrite = $true
        }
    }
    if ($NeedsWrite) {
        New-Item -ItemType Directory -Force -Path $LibDir | Out-Null
        $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($HelperFile, $EmbeddedContent, $Utf8NoBom)
    }
    return $HelperFile
}
try { . (Initialize-ClibmEnvPathHelper) } catch { Invoke-Expression $script:ClibmEmbeddedHelperContent }
# <<< clibm-env-path (generated) <<<

$BinName = "clibm-plugin-template"
if (Test-Path -LiteralPath "Cargo.toml") {
    $CargoToml = Get-Content "Cargo.toml" -Raw -Encoding UTF8
    if (($CargoToml -match '(?m)^\s*name\s*=\s*"([^"]+)"') -and ($Matches[1] -ne "clibm")) {
        $BinName = $Matches[1]
    }
}

Write-Host "===================================================================="
Write-Host " $BinName -- Plugin Uninstaller (Windows PowerShell - UTF-8-BOM)"
Write-Host "===================================================================="

if (-not $Purge -and -not $Keep -and -not $Yes) {
    try {
        Write-Host "Select Uninstallation Mode for ${BinName}:"
        Write-Host "  1) Remove $BinName.exe binary only, keep plugin config [Default]"
        Write-Host "  2) Remove $BinName.exe binary AND purge ~/.clibm/$BinName-config.json (-Purge)"
        $c = Read-Host "Select option [1-2, default: 1]"
        if ($c -eq "2" -or $c -eq "purge") { $Purge = $true }
    } catch {
        $Purge = $false
    }
}

$CfgFile = Join-Path $HOME ".clibm\$BinName-config.json"
$UninstallOk = Uninstall-ClibmPlugin -Name $BinName
if (-not $UninstallOk) {
    Write-Host "==> Some files of $BinName could not be removed (see the warnings above)." -ForegroundColor Yellow
}

if ($Purge -and (Test-Path -LiteralPath $CfgFile)) {
    Remove-Item -LiteralPath $CfgFile -Force
    Write-Host "==> Purged $CfgFile"
}
Write-Host "==> $BinName uninstallation complete."
