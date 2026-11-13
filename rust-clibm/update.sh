#!/usr/bin/env bash
# ==============================================================================
# CLIBM -- Update Script (Linux / macOS)
# Encoding: UTF-8 (without BOM)
# ==============================================================================
# Updates CLIBM with `clibm update`, which rebuilds it from GitHub (or from this
# clone) and replaces the installed binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/update.sh | bash
#   ./update.sh [--plugins] [--check] [--repo <url>] [--branch <name>]
#
# - Inside a clone of the repository: pulls the latest changes (git pull --ff-only)
#   and builds from the clone.
# - When clibm is not installed: runs the installer instead.

set -euo pipefail

PLUGINS=false
CHECK=false
REPO=""
BRANCH=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --plugins) PLUGINS=true; shift ;;
    --check) CHECK=true; shift ;;
    --repo) REPO="$2"; shift 2 ;;
    --branch) BRANCH="$2"; shift 2 ;;
    *) shift ;;
  esac
done

SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi
HOME_DIR="${CLIBM_HOME:-${HOME}}"

find_clibm() {
  local cfg dir
  if command -v clibm >/dev/null 2>&1; then
    command -v clibm
    return 0
  fi
  cfg="${HOME_DIR%/}/.clibm/install-config.json"
  if [ -f "$cfg" ]; then
    dir=$(sed -n 's/.*"install_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$cfg" | awk 'NR == 1')
    if [ -n "$dir" ] && [ -x "${dir}/clibm" ]; then
      printf '%s\n' "${dir}/clibm"
      return 0
    fi
  fi
  for dir in "${HOME_DIR%/}/.local/bin" "${HOME_DIR%/}/.clibm-bin" "${HOME_DIR%/}/.cargo/bin"; do
    if [ -x "${dir}/clibm" ]; then
      printf '%s\n' "${dir}/clibm"
      return 0
    fi
  done
  return 1
}

echo "============================================================"
echo " CLIBM Updater"
echo "============================================================"

if ! CLIBM_BIN=$(find_clibm); then
  echo "==> clibm is not installed; running the installer instead."
  BASE="${CLIBM_RAW_BASE:-https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm}"
  curl -fsSL "${BASE%/}/install.sh" | bash -s -- -y
  exit $?
fi

UPDATE_ARGS=(update)
if [ -n "$SCRIPT_DIR" ] && [ -f "${SCRIPT_DIR}/Cargo.toml" ] && [ -f "${SCRIPT_DIR}/src/main.rs" ] \
   && [ "$CHECK" = false ] && [ -z "$REPO" ] && [ -z "$BRANCH" ]; then
  if command -v git >/dev/null 2>&1; then
    echo "==> Updating the local clone (git pull --ff-only)..."
    git -C "$SCRIPT_DIR" pull --ff-only || echo "==> git pull failed; building the clone as it is."
  fi
  UPDATE_ARGS+=(--path "$SCRIPT_DIR")
else
  if [ -n "$REPO" ]; then UPDATE_ARGS+=(--repo "$REPO"); fi
  if [ -n "$BRANCH" ]; then UPDATE_ARGS+=(--branch "$BRANCH"); fi
fi
if [ "$CHECK" = true ]; then UPDATE_ARGS+=(--check); fi
if [ "$PLUGINS" = true ]; then UPDATE_ARGS+=(--plugins); fi

"$CLIBM_BIN" "${UPDATE_ARGS[@]}"
echo "==> Done. Verify with: clibm --version"
