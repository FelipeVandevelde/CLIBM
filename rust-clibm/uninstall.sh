#!/usr/bin/env bash
# ==============================================================================
# CLIBM v0.1.0 — Cross-Platform Uninstallation Script (Linux / macOS)
# Encoding: UTF-8 with BOM (EF BB BF) for cross-platform compatibility
# ==============================================================================
# Usage:
#   ./uninstall.sh           # Interactive choice or removes binaries (preserves .clibm data)
#   ./uninstall.sh --keep    # Removes clibm + plugin binaries, preserves .clibm data
#   ./uninstall.sh --purge   # Removes binaries AND deletes local .clibm repository data

set -euo pipefail

PURGE=""
NON_INTERACTIVE=false

for arg in "$@"; do
  case "$arg" in
    --purge|-p)
      PURGE="true"
      ;;
    --keep|-k)
      PURGE="false"
      ;;
    -y|--yes)
      NON_INTERACTIVE=true
      ;;
  esac
done

echo "============================================================"
echo " CLIBM v0.1.0 Uninstaller (Linux / macOS · UTF-8-BOM)"
echo "============================================================"

if [ -z "$PURGE" ]; then
  if [ "$NON_INTERACTIVE" = true ]; then
    PURGE="false"
  else
    echo ""
    echo "Select Uninstallation Mode:"
    echo "  1) Standard Uninstall — Remove clibm & plugin binaries, KEEP .clibm project data [Default]"
    echo "  2) Full Purge (--purge) — Remove binaries AND delete .clibm repository data"
    read -r -p "Select option [1-2, default: 1]: " choice_uninstall
    case "${choice_uninstall:-1}" in
      2|purge) PURGE="true" ;;
      *) PURGE="false" ;;
    esac
  fi
fi

GLOBAL_CFG_DIR="${HOME}/.clibm"
INSTALL_CFG_FILE="${GLOBAL_CFG_DIR}/install-config.json"
CFG_INSTALL_DIR=""
CFG_PATH_ADDED="false"

if [ -f "$INSTALL_CFG_FILE" ]; then
  BOM=$'\xef\xbb\xbf'
  CLEAN_CFG=$(tr -d '\r' < "$INSTALL_CFG_FILE" | sed "1s/^${BOM}//")
  CFG_INSTALL_DIR=$(printf '%s\n' "$CLEAN_CFG" | sed -n 's/.*"install_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)
  CFG_INSTALL_DIR="${CFG_INSTALL_DIR/#\~/$HOME}"
  PARSED_ADDED=$(printf '%s\n' "$CLEAN_CFG" | sed -n 's/.*"path_entry_added"[[:space:]]*:[[:space:]]*\(true\|false\).*/\1/p' | head -n1)
  if [ -n "$PARSED_ADDED" ]; then
    CFG_PATH_ADDED="$PARSED_ADDED"
  fi
fi

HELPER_FILE="${GLOBAL_CFG_DIR}/lib/env-path.sh"
HELPER_LOADED=false
LEDGER_DIRS=""
if [ -f "$HELPER_FILE" ]; then
  # shellcheck disable=SC1090
  . "$HELPER_FILE"
  HELPER_LOADED=true
  LEDGER_DIRS=$(clibm_path_all_dirs || true)
else
  echo "-> Warning: Shared PATH helper (${HELPER_FILE}) not found; skipping user PATH ledger cleanup."
fi

CANDIDATE_LIST=$(
  {
    [ -n "$CFG_INSTALL_DIR" ] && printf '%s\n' "$CFG_INSTALL_DIR"
    [ -n "$LEDGER_DIRS" ] && printf '%s\n' "$LEDGER_DIRS"
    printf '%s\n' "${HOME}/.local/bin"
    printf '%s\n' "${CARGO_HOME:-$HOME/.cargo}/bin"
    printf '%s\n' "${HOME}/.clibm-bin"
  } | awk 'NF && !seen[$0]++'
)

REMOVED_COUNT=0

while IFS= read -r bin_dir; do
  [ -z "$bin_dir" ] && continue
  if [ -d "$bin_dir" ]; then
    for bin_path in "$bin_dir"/clibm "$bin_dir"/clibm-*; do
      if [ -f "$bin_path" ]; then
        echo "-> Removing binary: $bin_path"
        rm -f "$bin_path"
        REMOVED_COUNT=$((REMOVED_COUNT + 1))
      fi
    done
  fi
done <<EOF
$CANDIDATE_LIST
EOF

if [ "$REMOVED_COUNT" -eq 0 ]; then
  echo "-> No clibm binaries found in configured or standard directories"
else
  echo "-> Removed $REMOVED_COUNT binary(ies)"
fi

PATH_REMOVED_COUNT=0
REMOVED_PATH_LIST=""
if [ "$HELPER_LOADED" = true ]; then
  clibm_path_remove_all
  PATH_REMOVED_COUNT="$CLIBM_REMOVED_PATH_COUNT"
  REMOVED_PATH_LIST="$CLIBM_REMOVED_PATH_DIRS"

  if [ "$CFG_PATH_ADDED" = "true" ] && [ -n "$CFG_INSTALL_DIR" ]; then
    if clibm_remove_profile_block "$CFG_INSTALL_DIR"; then
      PATH_REMOVED_COUNT=$((PATH_REMOVED_COUNT + 1))
      REMOVED_PATH_LIST="${REMOVED_PATH_LIST}${CFG_INSTALL_DIR}"$'\n'
    fi
    clibm_remove_dir_from_session_path "$CFG_INSTALL_DIR"
  fi

  if [ -n "$REMOVED_PATH_LIST" ]; then
    while IFS= read -r rdir; do
      [ -z "$rdir" ] && continue
      echo "-> Removed user PATH entry: $rdir"
    done <<EOF
$REMOVED_PATH_LIST
EOF
  fi
fi

EMPTY_CHECK_LIST=$(
  {
    [ -n "$REMOVED_PATH_LIST" ] && printf '%s\n' "$REMOVED_PATH_LIST"
    [ -n "$CFG_INSTALL_DIR" ] && printf '%s\n' "$CFG_INSTALL_DIR"
    printf '%s\n' "${HOME}/.clibm-bin"
  } | awk 'NF && !seen[$0]++'
)

while IFS= read -r edir; do
  [ -z "$edir" ] && continue
  if [ -d "$edir" ] && [ -z "$(ls -A "$edir" 2>/dev/null || true)" ]; then
    rmdir "$edir" 2>/dev/null || true
    echo "-> Removed empty install directory: $edir"
  fi
done <<EOF
$EMPTY_CHECK_LIST
EOF

if [ -d "${GLOBAL_CFG_DIR}/lib" ]; then
  rm -rf "${GLOBAL_CFG_DIR}/lib"
  echo "-> Removed shared helper library: ${GLOBAL_CFG_DIR}/lib"
fi

if [ "$PURGE" = "true" ]; then
  if [ -d ".clibm" ]; then
    echo "-> [--purge] Removing local .clibm/ CAS repository directory..."
    rm -rf ".clibm"
    echo "-> Deleted .clibm/ repository data."
  else
    echo "-> [--purge] No .clibm/ directory found in current working directory."
  fi
  if [ -f "$INSTALL_CFG_FILE" ]; then
    rm -f "$INSTALL_CFG_FILE"
    echo "-> [--purge] Deleted ${INSTALL_CFG_FILE}"
  fi
else
  echo "-> Preserved local .clibm/ repository data (pass --purge to remove data)."
fi

# Write UTF-8-BOM uninstallation receipt log
printf '\xEF\xBB\xBF{"uninstalled_binaries":%d,"purged_data":%s,"path_entries_removed":%d,"encoding":"UTF-8-BOM"}\n' \
  "$REMOVED_COUNT" "$PURGE" "$PATH_REMOVED_COUNT" > "${HOME}/.clibm-uninstall-receipt.json" 2>/dev/null || true

echo "============================================================"
echo " Uninstallation complete."
echo "============================================================"
