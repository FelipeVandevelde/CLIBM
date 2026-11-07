# ==============================================================================
# clibm-dep-audit — Standalone Plugin Installer (Windows PowerShell)
# Encoding: UTF-8 with BOM (EF BB BF) for Windows PowerShell compatibility
# ==============================================================================
param (
    [string]$Hooks = "",
    [string]$InstallDir = "",
    [string]$RawBaseUrl = "",
    [switch]$NoModifyPath = $false,
    [switch]$Yes = $false
)
$ErrorActionPreference = "Stop"
$CallerDir = (Get-Location).ProviderPath
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { $CallerDir }
if ($PSScriptRoot) {
    Set-Location -Path $PSScriptRoot
}

# >>> clibm-env-path (generated) >>>
function Initialize-ClibmEnvPathHelper {
    try { Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction SilentlyContinue } catch {}
    $HomeDir = if ($env:CLIBM_HOME) { $env:CLIBM_HOME } elseif ($HOME) { $HOME } elseif ($env:USERPROFILE) { $env:USERPROFILE } else { [Environment]::GetFolderPath('UserProfile') }
    $LibDir = Join-Path $HomeDir '.clibm\lib'
    $HelperFile = Join-Path $LibDir 'env-path.ps1'
    $EmbeddedVersion = 3
    $EmbeddedContent = @'
# clibm-env-path-version: 3
# ==============================================================================
# CLIBM Shared User PATH Helper (Windows PowerShell 5.1 & 7+)
# Encoding: UTF-8 with BOM (EF BB BF)
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

function Update-ClibmRegistryFile {
    param(
        [Parameter(Mandatory = $true)][string]$RegFile,
        [Parameter(Mandatory = $true)][PSCustomObject]$NewEntry
    )
    $RegParent = Split-Path -Parent $RegFile
    if ($RegParent -and -not (Test-Path -LiteralPath $RegParent)) {
        New-Item -ItemType Directory -Force -Path $RegParent | Out-Null
    }
    $List = @()
    if (Test-Path -LiteralPath $RegFile) {
        try {
            $Raw = [System.IO.File]::ReadAllText($RegFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF).Trim()
            if ($Raw -and $Raw -ne '[]') {
                $Parsed = $Raw | ConvertFrom-Json
                if ($null -ne $Parsed) {
                    $List = @($Parsed)
                }
            }
        } catch {
            $List = @()
        }
    }
    $Updated = @()
    $Found = $false
    foreach ($Item in $List) {
        if ($null -eq $Item -or [string]::IsNullOrWhiteSpace($Item.name)) { continue }
        if ($Item.name -eq $NewEntry.name) {
            $Updated += $NewEntry
            $Found = $true
        } else {
            $ExistingHooks = @()
            if ($null -ne $Item.hooks) {
                foreach ($H in @($Item.hooks)) {
                    if (-not [string]::IsNullOrWhiteSpace([string]$H)) {
                        $ExistingHooks += [string]$H
                    }
                }
            }
            if ($ExistingHooks.Count -eq 0) { $ExistingHooks = @('manual-run') }
            $Updated += [PSCustomObject][ordered]@{
                name        = [string]$Item.name
                version     = if ($Item.version) { [string]$Item.version } else { '0.1.0' }
                description = if ($Item.description) { [string]$Item.description } else { '' }
                enabled     = if ($null -ne $Item.enabled) { [bool]$Item.enabled } else { $true }
                hooks       = [string[]]$ExistingHooks
                executable  = if ($Item.executable) { ([string]$Item.executable -replace '\\', '/') } else { $null }
            }
        }
    }
    if (-not $Found) {
        $Updated += $NewEntry
    }
    if ($Updated.Count -eq 0) {
        $Json = '[]'
    } else {
        $ItemJsonBlocks = foreach ($Entry in $Updated) {
            $HooksArr = @($Entry.hooks | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
            if ($HooksArr.Count -eq 0) { $HooksArr = @('manual-run') }
            $HooksJson = ($HooksArr | ForEach-Object { '"' + (([string]$_ -replace '\\', '\\') -replace '"', '\"') + '"' }) -join ', '
            $EscName = ([string]$Entry.name -replace '\\', '\\') -replace '"', '\"'
            $EscVer  = ([string]$Entry.version -replace '\\', '\\') -replace '"', '\"'
            $EscDesc = ([string]$Entry.description -replace '\\', '\\') -replace '"', '\"'
            $EnBool  = if ($Entry.enabled) { 'true' } else { 'false' }
            $ExeField = if ($Entry.executable) {
                '"' + ((([string]$Entry.executable -replace '\\', '/') -replace '"', '\"')) + '"'
            } else {
                'null'
            }
            "  {`n    `"name`": `"$EscName`",`n    `"version`": `"$EscVer`",`n    `"description`": `"$EscDesc`",`n    `"enabled`": $EnBool,`n    `"hooks`": [$HooksJson],`n    `"executable`": $ExeField`n  }"
        }
        $Json = "[`n" + ($ItemJsonBlocks -join ",`n") + "`n]`n"
    }
    $Utf8Bom = New-Object System.Text.UTF8Encoding($true)
    [System.IO.File]::WriteAllText($RegFile, $Json, $Utf8Bom)
}

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

    $NewEntry = [PSCustomObject][ordered]@{
        name        = $Name
        version     = $Version
        description = $Description
        enabled     = $true
        hooks       = [string[]]$CleanHooks
        executable  = ($Executable -replace '\\', '/')
    }

    $CandidateRegFiles = @()
    $HomeDir = Get-ClibmHomeDirectory
    $GlobalPluginsDir = Join-Path (Join-Path $HomeDir '.clibm') 'plugins'
    $GlobalRegFile = Join-Path $GlobalPluginsDir 'registry.json'

    if ($Scope -ne 'project-only') {
        $CandidateRegFiles += $GlobalRegFile
    }

    $ProjectPluginDirs = @(Find-ClibmProjectPluginDirs -SearchRoots @($ProjectDir, $InstallDir))
    foreach ($PDir in $ProjectPluginDirs) {
        $CandidateRegFiles += (Join-Path $PDir 'registry.json')
    }

    if ($Scope -eq 'project-only' -and $ProjectPluginDirs.Count -eq 0) {
        $BaseProj = if (-not [string]::IsNullOrWhiteSpace($ProjectDir)) { $ProjectDir } else { (Get-Location).ProviderPath }
        if (-not [string]::IsNullOrWhiteSpace($BaseProj)) {
            $CandidateRegFiles += (Join-Path (Join-Path (Join-Path $BaseProj '.clibm') 'plugins') 'registry.json')
        }
        $CandidateRegFiles += $GlobalRegFile
    }

    $BinDir = if (-not [string]::IsNullOrWhiteSpace($InstallDir)) {
        $InstallDir
    } else {
        Split-Path -Parent $Executable
    }
    if (-not [string]::IsNullOrWhiteSpace($BinDir)) {
        $CandidateRegFiles += (Join-Path $BinDir 'registry.json')
    }

    $SeenReg = @{}
    foreach ($RegFile in $CandidateRegFiles) {
        if ([string]::IsNullOrWhiteSpace($RegFile)) { continue }
        $Comp = ConvertTo-ClibmComparablePath $RegFile
        if (-not $Comp -or $SeenReg.ContainsKey($Comp)) { continue }
        $SeenReg[$Comp] = $true
        Update-ClibmRegistryFile -RegFile $RegFile -NewEntry $NewEntry
        Write-Host "==> Registered $Name in plugin registry: $RegFile"
    }
}

function Unregister-ClibmPlugin {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [string]$ProjectDir = '',
        [string[]]$ExtraDirs = @()
    )
    $HomeDir = Get-ClibmHomeDirectory
    $CandidateFiles = @(
        (Join-Path (Join-Path (Join-Path $HomeDir '.clibm') 'plugins') 'registry.json'),
        (Join-Path (Join-Path $HomeDir '.clibm-bin') 'registry.json'),
        (Join-Path (Join-Path $HomeDir '.local\bin') 'registry.json')
    )
    foreach ($PDir in @(Find-ClibmProjectPluginDirs -SearchRoots (@($ProjectDir) + $ExtraDirs))) {
        $CandidateFiles += (Join-Path $PDir 'registry.json')
    }
    foreach ($EDir in $ExtraDirs) {
        if (-not [string]::IsNullOrWhiteSpace($EDir)) {
            $CandidateFiles += (Join-Path $EDir 'registry.json')
        }
    }
    $Seen = @{}
    foreach ($RegFile in $CandidateFiles) {
        if ([string]::IsNullOrWhiteSpace($RegFile)) { continue }
        $Comp = ConvertTo-ClibmComparablePath $RegFile
        if (-not $Comp -or $Seen.ContainsKey($Comp)) { continue }
        $Seen[$Comp] = $true
        if (-not (Test-Path -LiteralPath $RegFile)) { continue }
        try {
            $Raw = [System.IO.File]::ReadAllText($RegFile, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF).Trim()
            if (-not $Raw -or $Raw -eq '[]') { continue }
            $Parsed = $Raw | ConvertFrom-Json
            if ($null -eq $Parsed) { continue }
            $Kept = @()
            $Removed = $false
            foreach ($Item in @($Parsed)) {
                if ($null -eq $Item -or [string]::IsNullOrWhiteSpace($Item.name)) { continue }
                if ($Item.name -eq $Name) {
                    $Removed = $true
                } else {
                    $Kept += $Item
                }
            }
            if ($Removed) {
                $Utf8Bom = New-Object System.Text.UTF8Encoding($true)
                if ($Kept.Count -eq 0) {
                    [System.IO.File]::WriteAllText($RegFile, "[]`n", $Utf8Bom)
                } else {
                    $First = $Kept[0]
                    $Rest  = if ($Kept.Count -gt 1) { $Kept[1..($Kept.Count - 1)] } else { @() }
                    [System.IO.File]::WriteAllText($RegFile, "[]`n", $Utf8Bom)
                    foreach ($K in $Kept) {
                        $EntryObj = [PSCustomObject][ordered]@{
                            name        = [string]$K.name
                            version     = if ($K.version) { [string]$K.version } else { '0.1.0' }
                            description = if ($K.description) { [string]$K.description } else { '' }
                            enabled     = if ($null -ne $K.enabled) { [bool]$K.enabled } else { $true }
                            hooks       = @($K.hooks | Where-Object { $_ })
                            executable  = if ($K.executable) { ([string]$K.executable -replace '\\', '/') } else { $null }
                        }
                        Update-ClibmRegistryFile -RegFile $RegFile -NewEntry $EntryObj
                    }
                }
                Write-Host "==> Removed $Name from plugin registry: $RegFile"
            }
        } catch {
            # Ignore malformed registry files during uninstall
        }
    }
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
        $Utf8Bom = New-Object System.Text.UTF8Encoding($true)
        [System.IO.File]::WriteAllText($HelperFile, $EmbeddedContent, $Utf8Bom)
    }
    return $HelperFile
}
try { . (Initialize-ClibmEnvPathHelper) } catch { Invoke-Expression $script:ClibmEmbeddedHelperContent }
# <<< clibm-env-path (generated) <<<

Write-Host "===================================================================="
Write-Host " CLIBM-DEP-AUDIT v0.1.0 — DAG & Cycle Auditor Installer (UTF-8-BOM)"
Write-Host "===================================================================="

if (-not $Hooks) {
    if ($Yes) {
        $Hooks = "pre-commit,on-task-depend"
    } else {
        Write-Host "[1/2] Select Hook Mode for clibm-dep-audit:"
        Write-Host "  1) Automatic Hooks (pre-commit + on-task-depend) [Default]"
        Write-Host "  2) Manual Only (run via 'clibm plugin run dep-audit')"
        $c = Read-Host "Select option [1-2, default: 1]"
        if ($c -eq "2") { $Hooks = "manual-run" } else { $Hooks = "pre-commit,on-task-depend" }
    }
}

$HomeDir = Get-ClibmHomeDirectory
$DefaultBin = Join-Path $HomeDir ".clibm-bin"
if (-not $InstallDir) {
    if ($Yes) {
        $InstallDir = $DefaultBin
    } else {
        $d = Read-Host "[2/2] Specify installation directory [default: $DefaultBin]"
        if ([string]::IsNullOrWhiteSpace($d)) { $InstallDir = $DefaultBin } else { $InstallDir = $d }
    }
}

$InstallDir = Resolve-ClibmNormalizedDir $InstallDir
if (-not (Test-ClibmCoreInstalled -ExtraDir $InstallDir)) {
    Write-Host "==> Warning: CLIBM core installation not detected. clibm-dep-audit depends on CLIBM core at runtime; proceeding with standalone plugin installation..." -ForegroundColor Yellow
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$InstalledExe = Join-Path $InstallDir "clibm-dep-audit.exe"
$InstallMode = Invoke-ClibmBuildAndStage `
    -CrateName "clibm-dep-audit" `
    -RemoteSubPath "plugins/clibm-dep-audit" `
    -SourceFiles @("Cargo.toml", "src/main.rs") `
    -DestinationBinary $InstalledExe `
    -LocalCandidateDir $ScriptDir `
    -RawBaseUrl $RawBaseUrl

Register-ClibmGlobalPlugin `
    -Name "clibm-dep-audit" `
    -Description "Critical path and circular dependency analyzer for task graphs" `
    -Hooks @($Hooks -split ',' | Where-Object { $_ }) `
    -Executable $InstalledExe `
    -ProjectDir $CallerDir `
    -InstallDir $InstallDir

if (-not $NoModifyPath) {
    $PathRes = Add-ClibmPathEntry -Owner "clibm-dep-audit" -Dir $InstallDir
    Write-Host "==> User PATH ($InstallDir): $($PathRes.Status)"
} else {
    Write-Host "==> User PATH: skipped (-NoModifyPath)"
}
Write-Host "==> Installed clibm-dep-audit.exe ($InstallMode mode) to $InstallDir (hooks: $Hooks)"
