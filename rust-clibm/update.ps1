# ==============================================================================
# CLIBM v0.1.0 — GitHub Update Script (Windows PowerShell)
# Encoding: UTF-8 with BOM (EF BB BF) for Windows PowerShell compatibility
# ==============================================================================
# Pulls latest updates from the GitHub repository and reinstalls the core binary (and optional plugins).
# Usage:
#   .\update.ps1 [-Repo <github-url>] [-Branch <branch>] [-Plugins]

param (
    [string]$Repo = "https://github.com/clibm-org/clibm.git",
    [string]$Branch = "main",
    [switch]$Plugins = $false
)

$ErrorActionPreference = "Stop"

if (Test-Path ".clibm\config.json") {
    try {
        $Cfg = Get-Content ".clibm\config.json" -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($Cfg.github_repo_url -and $Repo -eq "https://github.com/clibm-org/clibm.git") {
            $Repo = $Cfg.github_repo_url
        }
        if ($Cfg.update_branch -and $Branch -eq "main") {
            $Branch = $Cfg.update_branch
        }
    } catch {}
}

Write-Host "============================================================"
Write-Host " CLIBM GitHub Updater (Windows PowerShell · UTF-8-BOM)"
Write-Host " Remote : $Repo"
Write-Host " Branch : $Branch"
Write-Host "============================================================"

if ((Test-Path ".git") -and (Test-Path "Cargo.toml")) {
    Write-Host "-> Detected local Git clone of CLIBM source repository."
    Write-Host "-> Running: git fetch origin $Branch && git pull origin $Branch"
    git fetch origin $Branch
    git pull origin $Branch
    Write-Host "-> Rebuilding and installing core binary..."
    cargo install --path . --force
} else {
    Write-Host "-> Installing latest clibm core directly from GitHub repository..."
    cargo install --git $Repo --branch $Branch --force
}

if ($Plugins -and (Test-Path "plugins")) {
    Write-Host "-> Updating standalone plugins in .\plugins\..."
    Get-ChildItem -Path "plugins" -Directory | ForEach-Object {
        $CargoFile = Join-Path $_.FullName "Cargo.toml"
        if (Test-Path $CargoFile) {
            Write-Host "   * Building and installing plugin from $($_.Name)..."
            Push-Location $_.FullName
            cargo install --path . --force
            Pop-Location
        }
    }
}

Write-Host "============================================================"
Write-Host " Update complete! Verify with: clibm --version"
Write-Host "============================================================"
