#!/usr/bin/env bash
# ==============================================================================
# CLIBM v0.1.0 -- GitHub Update Script (Linux / macOS)
# Encoding: UTF-8 (without BOM) for cross-platform compatibility
# ==============================================================================
# Pulls latest updates from the GitHub repository and reinstalls the core binary (and optional plugins).
# Usage:
#   ./update.sh [--repo <github-url>] [--branch <branch>] [--plugins]

set -euo pipefail

REPO_URL="${CLIBM_GITHUB_REPO:-https://github.com/FelipeVandevelde/CLIBM.git}"
BRANCH="main"
UPDATE_PLUGINS=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO_URL="$2"
      shift 2
      ;;
    --branch)
      BRANCH="$2"
      shift 2
      ;;
    --plugins)
      UPDATE_PLUGINS=true
      shift
      ;;
    *)
      shift
      ;;
  esac
done

# Read repository URL and branch from .clibm/config.json if present and not overridden
if [ -f ".clibm/config.json" ] && command -v grep >/dev/null 2>&1; then
  CFG_REPO=$(grep -o '"github_repo_url"[[:space:]]*:[[:space:]]*"[^"]*"' .clibm/config.json | sed 's/.*"github_repo_url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || true)
  CFG_BRANCH=$(grep -o '"update_branch"[[:space:]]*:[[:space:]]*"[^"]*"' .clibm/config.json | sed 's/.*"update_branch"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || true)
  if [ -n "$CFG_REPO" ]; then REPO_URL="$CFG_REPO"; fi
  if [ -n "$CFG_BRANCH" ]; then BRANCH="$CFG_BRANCH"; fi
fi

echo "============================================================"
echo " CLIBM GitHub Updater (Linux / macOS - UTF-8-BOM)"
echo " Remote : $REPO_URL"
echo " Branch : $BRANCH"
echo "============================================================"

if [ -d ".git" ] && [ -f "Cargo.toml" ]; then
  echo "-> Detected local Git clone of CLIBM source repository."
  echo "-> Running: git fetch origin $BRANCH && git pull origin $BRANCH"
  git fetch origin "$BRANCH"
  git pull origin "$BRANCH"
  echo "-> Rebuilding and installing core binary..."
  cargo install --path . --force
else
  echo "-> Installing latest clibm core directly from GitHub repository..."
  cargo install --git "$REPO_URL" --branch "$BRANCH" --force
fi

if [ "$UPDATE_PLUGINS" = true ] && [ -d "plugins" ]; then
  echo "-> Updating standalone plugins in ./plugins/..."
  for plugin_dir in plugins/*/; do
    case "$plugin_dir" in *plugin-template*) continue ;; esac
    if [ -f "${plugin_dir}install.sh" ]; then
      echo "   * Building, installing, and registering plugin from ${plugin_dir}..."
      (cd "$plugin_dir" && ./install.sh -y)
    elif [ -f "${plugin_dir}Cargo.toml" ]; then
      echo "   * Building and installing plugin from ${plugin_dir}..."
      (cd "$plugin_dir" && cargo install --path . --force)
    fi
  done
fi

echo "============================================================"
echo " Update complete! Verify with: clibm --version"
echo "============================================================"
