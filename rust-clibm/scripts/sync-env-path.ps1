# ==============================================================================
# CLIBM Shared PATH Helper Synchronizer (Windows PowerShell 5.1 & 7+)
# Encoding: UTF-8 with BOM (EF BB BF)
# Syncs rust-clibm/scripts/env-path.{ps1,sh} into the embedded marker blocks of
# all standalone plugin scripts and verifies src/plugin.rs scaffolding.
# Usage:
#   .\scripts\sync-env-path.ps1          # Update embedded blocks + ensure BOM
#   .\scripts\sync-env-path.ps1 -Check   # Fail (exit 1) if any copy drifted
# ==============================================================================

param (
    [switch]$Check = $false
)

$ErrorActionPreference = 'Stop'

$ScriptsDir = $PSScriptRoot
$RustRoot = Split-Path -Parent $ScriptsDir
$Ps1HelperPath = Join-Path $ScriptsDir 'env-path.ps1'
$ShHelperPath  = Join-Path $ScriptsDir 'env-path.sh'
$Utf8Bom = New-Object System.Text.UTF8Encoding($true)

function Read-CleanText {
    param([string]$FilePath)
    $Raw = [System.IO.File]::ReadAllText($FilePath, [System.Text.Encoding]::UTF8)
    $Clean = $Raw.TrimStart([char]0xFEFF)
    return ($Clean -replace "`r`n", "`n").TrimEnd("`n")
}

function Test-HasUtf8Bom {
    param([string]$FilePath)
    $Bytes = [System.IO.File]::ReadAllBytes($FilePath)
    return ($Bytes.Length -ge 3 -and $Bytes[0] -eq 0xEF -and $Bytes[1] -eq 0xBB -and $Bytes[2] -eq 0xBF)
}

function Get-HelperVersion {
    param([string]$CleanContent, [string]$Label)
    foreach ($Line in ($CleanContent -split "`n" | Select-Object -First 5)) {
        if ($Line -match '^#\s*clibm-env-path-version:\s*(\d+)\s*$') {
            return [int]$Matches[1]
        }
    }
    throw "Missing '# clibm-env-path-version: <N>' header in $Label"
}

$Ps1Clean = Read-CleanText $Ps1HelperPath
$ShClean  = Read-CleanText $ShHelperPath
$Ps1Ver = Get-HelperVersion -CleanContent $Ps1Clean -Label 'env-path.ps1'
$ShVer  = Get-HelperVersion -CleanContent $ShClean  -Label 'env-path.sh'

if ($Ps1Ver -ne $ShVer) {
    throw "Version mismatch between env-path.ps1 ($Ps1Ver) and env-path.sh ($ShVer)"
}

$MarkerStart = '# >>> clibm-env-path (generated) >>>'
$MarkerEnd   = '# <<< clibm-env-path (generated) <<<'

$Ps1BlockLf = @(
    $MarkerStart
    'function Initialize-ClibmEnvPathHelper {'
    '    $HomeDir = if ($HOME) { $HOME } elseif ($env:USERPROFILE) { $env:USERPROFILE } else { [Environment]::GetFolderPath(''UserProfile'') }'
    '    $LibDir = Join-Path $HomeDir ''.clibm\lib'''
    '    $HelperFile = Join-Path $LibDir ''env-path.ps1'''
    "    `$EmbeddedVersion = $Ps1Ver"
    '    $EmbeddedContent = @'''
    $Ps1Clean
    '''@'
    '    $NeedsWrite = $true'
    '    if (Test-Path -LiteralPath $HelperFile) {'
    '        try {'
    '            $HeadLines = Get-Content -LiteralPath $HelperFile -TotalCount 5 -Encoding UTF8 -ErrorAction Stop'
    '            foreach ($Line in $HeadLines) {'
    '                if ($Line -match ''clibm-env-path-version:\s*(\d+)'') {'
    '                    if ([int]$Matches[1] -ge $EmbeddedVersion) {'
    '                        $NeedsWrite = $false'
    '                    }'
    '                    break'
    '                }'
    '            }'
    '        } catch {'
    '            $NeedsWrite = $true'
    '        }'
    '    }'
    '    if ($NeedsWrite) {'
    '        New-Item -ItemType Directory -Force -Path $LibDir | Out-Null'
    '        $Utf8Bom = New-Object System.Text.UTF8Encoding($true)'
    '        [System.IO.File]::WriteAllText($HelperFile, $EmbeddedContent, $Utf8Bom)'
    '    }'
    '    return $HelperFile'
    '}'
    '. (Initialize-ClibmEnvPathHelper)'
    $MarkerEnd
) -join "`n"

$ShBlockLf = @(
    $MarkerStart
    'clibm_init_env_path_helper() {'
    '  local lib_dir="${HOME}/.clibm/lib"'
    '  local helper_file="${lib_dir}/env-path.sh"'
    "  local embedded_version=$ShVer"
    '  local needs_write=1'
    '  local bom=$''\xef\xbb\xbf'''
    '  if [ -f "$helper_file" ]; then'
    '    local first_three'
    '    first_three=$(head -c 3 "$helper_file" 2>/dev/null || true)'
    '    local existing_ver'
    '    existing_ver=$(tr -d ''\r'' < "$helper_file" | sed "1s/^${bom}//" | sed -n ''1,5s/^# clibm-env-path-version:[[:space:]]*\([0-9][0-9]*\).*/\1/p'' | head -n1)'
    '    if [ "$first_three" != "$bom" ] && [ -n "$existing_ver" ] && [ "$existing_ver" -ge "$embedded_version" ] 2>/dev/null; then'
    '      needs_write=0'
    '    fi'
    '  fi'
    '  if [ "$needs_write" -eq 1 ]; then'
    '    mkdir -p "$lib_dir"'
    '    cat <<''CLIBM_ENV_PATH_EOF'' | tr -d ''\r'' > "$helper_file"'
    $ShClean
    'CLIBM_ENV_PATH_EOF'
    '  fi'
    '  # shellcheck disable=SC1090'
    '  . "$helper_file"'
    '}'
    'clibm_init_env_path_helper'
    $MarkerEnd
) -join "`n"

$DriftedFiles = @()
$UpdatedFiles = @()

# Ensure canonical scripts & core lifecycle scripts have UTF-8 BOM
$CoreScripts = @(
    $Ps1HelperPath,
    $ShHelperPath,
    (Join-Path $ScriptsDir 'sync-env-path.ps1'),
    (Join-Path $RustRoot 'install.ps1'),
    (Join-Path $RustRoot 'install.sh'),
    (Join-Path $RustRoot 'uninstall.ps1'),
    (Join-Path $RustRoot 'uninstall.sh'),
    (Join-Path $RustRoot 'update.ps1'),
    (Join-Path $RustRoot 'update.sh')
)
foreach ($CS in $CoreScripts) {
    if (Test-Path -LiteralPath $CS) {
        if (-not (Test-HasUtf8Bom $CS)) {
            if ($Check) {
                $DriftedFiles += "$CS (missing UTF-8 BOM)"
            } else {
                $Txt = [System.IO.File]::ReadAllText($CS, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF)
                [System.IO.File]::WriteAllText($CS, $Txt, $Utf8Bom)
                $UpdatedFiles += "$CS (added UTF-8 BOM)"
            }
        }
    }
}

$PluginsRoot = Join-Path $RustRoot 'plugins'
$PluginDirs = Get-ChildItem -LiteralPath $PluginsRoot -Directory

$BlockRegex = [regex]::new(
    '(?ms)^# >>> clibm-env-path \(generated\) >>>.*?^# <<< clibm-env-path \(generated\) <<<'
)

$TargetFiles = @(
    (Join-Path $RustRoot 'install.ps1'),
    (Join-Path $RustRoot 'install.sh')
)
foreach ($PDir in $PluginDirs) {
    foreach ($ScriptName in @('install.ps1', 'uninstall.ps1', 'install.sh', 'uninstall.sh')) {
        $TargetFiles += (Join-Path $PDir.FullName $ScriptName)
    }
}

foreach ($FilePath in $TargetFiles) {
    if (-not (Test-Path -LiteralPath $FilePath)) { continue }

    $RawText = [System.IO.File]::ReadAllText($FilePath, [System.Text.Encoding]::UTF8).TrimStart([char]0xFEFF)
    $UsesCrlf = $RawText -match "`r`n"
    $NormLf = $RawText -replace "`r`n", "`n"

    if (-not $BlockRegex.IsMatch($NormLf)) {
        $DriftedFiles += "$FilePath (missing generated marker block)"
        continue
    }

    $DesiredBlock = if ($FilePath.EndsWith('.ps1')) { $Ps1BlockLf } else { $ShBlockLf }
    $ReplacedLf = $BlockRegex.Replace($NormLf, { param($m) $DesiredBlock }, 1)

    $FinalText = if ($UsesCrlf) { $ReplacedLf -replace "`n", "`r`n" } else { $ReplacedLf }
    $HasBom = Test-HasUtf8Bom $FilePath

    if (($ReplacedLf -cne $NormLf) -or (-not $HasBom)) {
        if ($Check) {
            $DriftedFiles += $FilePath
        } else {
            [System.IO.File]::WriteAllText($FilePath, $FinalText, $Utf8Bom)
            $UpdatedFiles += $FilePath
        }
    }
}

# Verify src/plugin.rs references canonical scripts via include_str!
$PluginRsPath = Join-Path $RustRoot 'src\plugin.rs'
if (Test-Path -LiteralPath $PluginRsPath) {
    $PluginRsContent = [System.IO.File]::ReadAllText($PluginRsPath, [System.Text.Encoding]::UTF8)
    if (($PluginRsContent -notmatch 'include_str!\("\.\./scripts/env-path\.ps1"\)') -or
        ($PluginRsContent -notmatch 'include_str!\("\.\./scripts/env-path\.sh"\)')) {
        $DriftedFiles += "$PluginRsPath (missing include_str! for ../scripts/env-path.{ps1,sh})"
    }
}

if ($Check) {
    if ($DriftedFiles.Count -gt 0) {
        Write-Host 'ERROR: Embedded env-path helper drift detected in:' -ForegroundColor Red
        foreach ($D in $DriftedFiles) {
            Write-Host "  - $D" -ForegroundColor Red
        }
        exit 1
    }
    Write-Host "OK: All embedded env-path helpers match canonical version $Ps1Ver (0 drift)."
    exit 0
} else {
    if ($DriftedFiles.Count -gt 0) {
        Write-Host 'WARNING: Some files lacked marker blocks or include_str!:' -ForegroundColor Yellow
        foreach ($D in $DriftedFiles) {
            Write-Host "  - $D" -ForegroundColor Yellow
        }
    }
    Write-Host "Synced env-path v$Ps1Ver across $($UpdatedFiles.Count) file(s)."
}
