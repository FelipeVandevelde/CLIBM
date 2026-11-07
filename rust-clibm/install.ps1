# ==============================================================================
# CLIBM v0.1.0 — Interactive Core Installer (Windows PowerShell)
# Encoding: UTF-8 with BOM (EF BB BF) for Windows PowerShell 5.1 & 7+ compatibility
# Supports BOTH:
#   - Remote / Raw GitHub Install:
#     irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.ps1 | iex
#   - Local Repository Install:
#     cd rust-clibm ; .\install.ps1
# ==============================================================================
# Presents interactive configuration options during installation:
#   1. Core Mode: Pure Core (without plugins) vs. Core + Default Plugins
#      (clibm-dep-audit, clibm-export-md, clibm-workflow-lint)
#   2. Language Selection: en (English), pt-BR (Português), es (Español)
#   3. Automatic Per-Modification Commits: true (default) or false
#   4. Machine Storage / Install Location: custom path (default: ~\.clibm-bin)
#   5. User PATH Registration: add install directory to user PATH (or -NoModifyPath)

param (
    [string]$Profile = "",
    [string]$Language = "",
    [string]$AutoCommit = "",
    [string]$InstallDir = "",
    [string]$RawBaseUrl = "",
    [switch]$NoModifyPath = $false,
    [switch]$Yes = $false
)

$ErrorActionPreference = "Stop"
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).ProviderPath }
if ($PSScriptRoot) {
    Set-Location -Path $PSScriptRoot
}

# >>> clibm-env-path (generated) >>>
function Initialize-ClibmEnvPathHelper {
    $HomeDir = if ($HOME) { $HOME } elseif ($env:USERPROFILE) { $env:USERPROFILE } else { [Environment]::GetFolderPath('UserProfile') }
    $LibDir = Join-Path $HomeDir '.clibm\lib'
    $HelperFile = Join-Path $LibDir 'env-path.ps1'
    $EmbeddedVersion = 2
    $EmbeddedContent = @'
# clibm-env-path-version: 2
# ==============================================================================
# CLIBM Shared User PATH Helper (Windows PowerShell 5.1 & 7+)
# Encoding: UTF-8 with BOM (EF BB BF)
# Manages user-scope PATH entries and reference-counted ownership in
# ~/.clibm/path-ledger without touching machine/system environment variables.
# ==============================================================================

function Get-ClibmHomeDirectory {
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

function Invoke-ClibmBuildAndStage {
    param(
        [Parameter(Mandatory = $true)][string]$CrateName,
        [AllowEmptyString()][string]$RemoteSubPath = '',
        [Parameter(Mandatory = $true)][string[]]$SourceFiles,
        [Parameter(Mandatory = $true)][string]$DestinationBinary,
        [string]$LocalCandidateDir = '',
        [string]$RawBaseUrl = ''
    )
    $DestDir = Split-Path -Parent $DestinationBinary
    if ($DestDir -and -not (Test-Path -LiteralPath $DestDir)) {
        New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
    }

    $LocalDir = Resolve-ClibmLocalSourceDir -CrateName $CrateName -RemoteSubPath $RemoteSubPath -CandidateDir $LocalCandidateDir
    if (-not [string]::IsNullOrWhiteSpace($LocalDir)) {
        Write-Host "==> [Local Mode] Building $CrateName from local source ($LocalDir)..."
        Push-Location -LiteralPath $LocalDir
        try {
            cargo build --release
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build --release failed for $CrateName (exit code $LASTEXITCODE)"
            }
            $BuiltBin = Join-Path $LocalDir "target\release\$CrateName.exe"
            if (-not (Test-Path -LiteralPath $BuiltBin)) {
                throw "Compiled binary not found at $BuiltBin"
            }
            Copy-Item -LiteralPath $BuiltBin -Destination $DestinationBinary -Force
        } finally {
            Pop-Location
        }
        return 'local'
    }

    $Base = if (-not [string]::IsNullOrWhiteSpace($RawBaseUrl)) {
        $RawBaseUrl.TrimEnd('/')
    } else {
        $RootBase = Get-ClibmRawRepoBase
        if ([string]::IsNullOrWhiteSpace($RemoteSubPath)) {
            $RootBase
        } else {
            "$RootBase/$($RemoteSubPath.Trim('/'))"
        }
    }

    $TempBuildDir = Join-Path ([System.IO.Path]::GetTempPath()) ("$CrateName-build-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $TempBuildDir | Out-Null
    Write-Host "==> [Remote Mode] Fetching $CrateName source files from GitHub ($Base)..."
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        foreach ($RelFile in $SourceFiles) {
            $CleanRel = $RelFile.TrimStart('/')
            $DestFile = Join-Path $TempBuildDir ($CleanRel -replace '/', '\')
            $DestParent = Split-Path -Parent $DestFile
            if ($DestParent -and -not (Test-Path -LiteralPath $DestParent)) {
                New-Item -ItemType Directory -Force -Path $DestParent | Out-Null
            }
            $FileUrl = "$Base/$CleanRel"
            Invoke-WebRequest -Uri $FileUrl -OutFile $DestFile -UseBasicParsing
        }

        Write-Host "==> Building $CrateName (Release Mode)..."
        Push-Location -LiteralPath $TempBuildDir
        try {
            cargo build --release
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build --release failed for $CrateName (exit code $LASTEXITCODE)"
            }
            $BuiltBin = Join-Path $TempBuildDir "target\release\$CrateName.exe"
            if (-not (Test-Path -LiteralPath $BuiltBin)) {
                throw "Compiled binary not found at $BuiltBin"
            }
            Copy-Item -LiteralPath $BuiltBin -Destination $DestinationBinary -Force
        } finally {
            Pop-Location
        }
    } finally {
        if (Test-Path -LiteralPath $TempBuildDir) {
            Remove-Item -LiteralPath $TempBuildDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    return 'remote'
}

function Register-ClibmGlobalPlugin {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Description,
        [Parameter(Mandatory = $true)][string[]]$Hooks,
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$Version = '0.1.0'
    )
    $HomeDir = Get-ClibmHomeDirectory
    $PluginsDir = Join-Path (Join-Path $HomeDir '.clibm') 'plugins'
    New-Item -ItemType Directory -Force -Path $PluginsDir | Out-Null
    $RegFile = Join-Path $PluginsDir 'registry.json'
    $List = @()
    if (Test-Path -LiteralPath $RegFile) {
        try {
            $Raw = [System.IO.File]::ReadAllText($RegFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF)
            $Parsed = $Raw | ConvertFrom-Json
            if ($null -ne $Parsed) {
                $List = @($Parsed)
            }
        } catch {
            $List = @()
        }
    }
    $Updated = @()
    $Found = $false
    $NewEntry = [PSCustomObject][ordered]@{
        name        = $Name
        version     = $Version
        description = $Description
        enabled     = $true
        hooks       = @($Hooks)
        executable  = ($Executable -replace '\\', '/')
    }
    foreach ($Item in $List) {
        if ($null -eq $Item) { continue }
        if ($Item.name -eq $Name) {
            $Updated += $NewEntry
            $Found = $true
        } else {
            $Updated += $Item
        }
    }
    if (-not $Found) {
        $Updated += $NewEntry
    }
    $Json = ConvertTo-Json -InputObject @($Updated) -Depth 5
    if (-not $Json.TrimStart().StartsWith('[')) {
        $Json = "[$Json]"
    }
    $Utf8Bom = New-Object System.Text.UTF8Encoding($true)
    [System.IO.File]::WriteAllText($RegFile, $Json, $Utf8Bom)
}
'@
    $NeedsWrite = $true
    if (Test-Path -LiteralPath $HelperFile) {
        try {
            $HeadLines = Get-Content -LiteralPath $HelperFile -TotalCount 5 -Encoding UTF8 -ErrorAction Stop
            foreach ($Line in $HeadLines) {
                if ($Line -match 'clibm-env-path-version:\s*(\d+)') {
                    if ([int]$Matches[1] -ge $EmbeddedVersion) {
                        $NeedsWrite = $false
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
        $Utf8Bom = New-Object System.Text.UTF8Encoding($true)
        [System.IO.File]::WriteAllText($HelperFile, $EmbeddedContent, $Utf8Bom)
    }
    return $HelperFile
}
. (Initialize-ClibmEnvPathHelper)
# <<< clibm-env-path (generated) <<<

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
        Write-Host "  2) Core + Default Plugins (builds dep-audit, export-md, workflow-lint)"
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

$HomeDir = Get-ClibmHomeDirectory
$DefaultBin = Join-Path $HomeDir ".clibm-bin"
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

$CoreSourceFiles = @(
    "Cargo.toml",
    "scripts/env-path.ps1",
    "scripts/env-path.sh",
    "src/main.rs",
    "src/lib.rs",
    "src/cli.rs",
    "src/canonical.rs",
    "src/cas.rs",
    "src/models.rs",
    "src/index.rs",
    "src/versioning.rs",
    "src/plugin.rs",
    "src/engine.rs"
)

$InstallMode = Invoke-ClibmBuildAndStage `
    -CrateName "clibm" `
    -RemoteSubPath "" `
    -SourceFiles $CoreSourceFiles `
    -DestinationBinary (Join-Path $InstallDir "clibm.exe") `
    -LocalCandidateDir $ScriptDir `
    -RawBaseUrl $RawBaseUrl

Write-Host "==> Installed pure clibm.exe binary ($InstallMode mode) to $InstallDir\clibm.exe"

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
Write-Host "   Install Mode   : $InstallMode"
Write-Host "   Profile        : $Profile"
Write-Host "   Language       : $Language"
Write-Host "   Auto-Commit    : $AutoCommit"
Write-Host "   Install Path   : $InstallDir"
Write-Host "   User PATH      : $PathStatus"
Write-Host "--------------------------------------------------------------------"

$GlobalCfgDir = Join-Path $HomeDir ".clibm"
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
    Write-Host "==> Installing default standalone plugins (clibm-dep-audit, clibm-export-md, clibm-workflow-lint)..."
    $EffectiveRawBase = if ($RawBaseUrl) { $RawBaseUrl.TrimEnd('/') } else { Get-ClibmRawRepoBase }
    foreach ($PluginName in @("clibm-dep-audit", "clibm-export-md", "clibm-workflow-lint")) {
        $LocalPluginDir = Resolve-ClibmLocalSourceDir -CrateName $PluginName -RemoteSubPath "plugins/$PluginName" -CandidateDir $ScriptDir
        $LocalPluginPs1 = if ($LocalPluginDir) { Join-Path $LocalPluginDir "install.ps1" } else { "" }
        if ($LocalPluginPs1 -and (Test-Path -LiteralPath $LocalPluginPs1)) {
            Push-Location -LiteralPath $LocalPluginDir
            try {
                if ($ModifyPath) {
                    & .\install.ps1 -InstallDir $InstallDir -Yes
                } else {
                    & .\install.ps1 -InstallDir $InstallDir -Yes -NoModifyPath
                }
            } finally {
                Pop-Location
            }
        } else {
            $RemotePluginPs1Url = "$EffectiveRawBase/plugins/$PluginName/install.ps1"
            $TempPluginScript = Join-Path ([System.IO.Path]::GetTempPath()) ("$PluginName-install-" + [guid]::NewGuid().ToString('N') + ".ps1")
            try {
                [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
                Invoke-WebRequest -Uri $RemotePluginPs1Url -OutFile $TempPluginScript -UseBasicParsing
                if ($ModifyPath) {
                    & $TempPluginScript -InstallDir $InstallDir -Yes -RawBaseUrl "$EffectiveRawBase/plugins/$PluginName"
                } else {
                    & $TempPluginScript -InstallDir $InstallDir -Yes -NoModifyPath -RawBaseUrl "$EffectiveRawBase/plugins/$PluginName"
                }
            } finally {
                if (Test-Path -LiteralPath $TempPluginScript) {
                    Remove-Item -LiteralPath $TempPluginScript -Force -ErrorAction SilentlyContinue
                }
            }
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
