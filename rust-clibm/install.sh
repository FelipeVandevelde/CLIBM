#!/usr/bin/env bash
# ==============================================================================
# CLIBM v0.1.0 — Interactive Core Installer (Linux / macOS)
# Encoding: UTF-8 with BOM (EF BB BF) for cross-platform compatibility
# ==============================================================================
# Presents interactive configuration options during installation:
#   1. Core Mode: Pure Core (without plugins) vs. Core + Default Plugins
#      (clibm-dep-audit, clibm-export-md, clibm-workflow-lint from ./plugins/)
#   2. Language Selection: en (English), pt-BR (Português), es (Español)
#   3. Automatic Per-Modification Commits: true (default) or false
#   4. Machine Storage / Install Location: custom path (default: ~/.local/bin)
#   5. User PATH Registration: add install directory to user PATH (or --no-modify-path)
#
# Also supports non-interactive flags:
#   ./install.sh --pure|--with-defaults --lang en|pt-BR|es --auto-commit true|false --install-dir <PATH> [--no-modify-path]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PROFILE=""
LANGUAGE=""
AUTO_COMMIT=""
INSTALL_DIR=""
NO_MODIFY_PATH=false
NON_INTERACTIVE=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pure)
      PROFILE="pure"
      shift
      ;;
    --with-defaults)
      PROFILE="with-defaults"
      shift
      ;;
    --lang|--language)
      LANGUAGE="$2"
      shift 2
      ;;
    --auto-commit)
      AUTO_COMMIT="$2"
      shift 2
      ;;
    --install-dir|--path|--install-path)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --no-modify-path)
      NO_MODIFY_PATH=true
      shift
      ;;
    -y|--yes)
      NON_INTERACTIVE=true
      shift
      ;;
    *)
      shift
      ;;
  esac
done

clibm_install_env_path_helper() {
  local src_helper="${SCRIPT_DIR}/scripts/env-path.sh"
  local lib_dir="${HOME}/.clibm/lib"
  local target_helper="${lib_dir}/env-path.sh"
  local bom=$'\xef\xbb\xbf'

  if [ ! -f "$src_helper" ]; then
    echo "ERROR: Required shared PATH helper not found at ${src_helper}" >&2
    exit 1
  fi

  local src_ver=1
  local parsed_src
  parsed_src=$(tr -d '\r' < "$src_helper" | sed "1s/^${bom}//" | sed -n '1,5s/^# clibm-env-path-version:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n1)
  if [ -n "$parsed_src" ]; then
    src_ver="$parsed_src"
  fi

  local needs_copy=1
  if [ -f "$target_helper" ]; then
    local first_three
    first_three=$(head -c 3 "$target_helper" 2>/dev/null || true)
    local existing_ver
    existing_ver=$(tr -d '\r' < "$target_helper" | sed "1s/^${bom}//" | sed -n '1,5s/^# clibm-env-path-version:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n1)
    if [ "$first_three" != "$bom" ] && [ -n "$existing_ver" ] && [ "$existing_ver" -ge "$src_ver" ] 2>/dev/null; then
      needs_copy=0
    fi
  fi

  if [ "$needs_copy" -eq 1 ]; then
    mkdir -p "$lib_dir"
    tr -d '\r' < "$src_helper" | sed "1s/^${bom}//" > "$target_helper"
  fi
  # shellcheck disable=SC1090
  . "$target_helper"
}

clibm_install_env_path_helper

echo "===================================================================="
echo " CLIBM v0.1.0 — Interactive Core Installation Wizard (UTF-8-BOM)"
echo "===================================================================="

# 1. Core Mode (Pure vs With Default Plugins)
if [ -z "$PROFILE" ]; then
  if [ "$NON_INTERACTIVE" = true ]; then
    PROFILE="pure"
  else
    echo ""
    echo "[1/5] Select CLIBM Core Installation Profile:"
    echo "  1) Pure Core (without plugins — 100% pure engine) [Default]"
    echo "  2) Core + Default Plugins (builds dep-audit, export-md, workflow-lint from ./plugins/)"
    read -r -p "Select option [1-2, default: 1]: " choice_profile
    case "${choice_profile:-1}" in
      2) PROFILE="with-defaults" ;;
      *) PROFILE="pure" ;;
    esac
  fi
fi

# 2. Language Selection
if [ -z "$LANGUAGE" ]; then
  if [ "$NON_INTERACTIVE" = true ]; then
    LANGUAGE="en"
  else
    echo ""
    echo "[2/5] Select Interface & CLI Language:"
    echo "  1) English (en) [Default]"
    echo "  2) Português (pt-BR)"
    echo "  3) Español (es)"
    read -r -p "Select language [1-3, default: 1]: " choice_lang
    case "${choice_lang:-1}" in
      2|pt|pt-BR) LANGUAGE="pt-BR" ;;
      3|es) LANGUAGE="es" ;;
      *) LANGUAGE="en" ;;
    esac
  fi
fi

# 3. Automatic Commits
if [ -z "$AUTO_COMMIT" ]; then
  if [ "$NON_INTERACTIVE" = true ]; then
    AUTO_COMMIT="true"
  else
    echo ""
    echo "[3/5] Enable Automatic Per-Modification Commits (auto_commit)?"
    echo "  1) Yes — Record every change individually as a BLAKE3 commit [Default]"
    echo "  2) No  — Require manual 'clibm commit -m ...'"
    read -r -p "Select option [1-2, default: 1]: " choice_auto
    case "${choice_auto:-1}" in
      2|n|no|false) AUTO_COMMIT="false" ;;
      *) AUTO_COMMIT="true" ;;
    esac
  fi
fi

# 4. Installation Directory on Machine
if [ -z "$INSTALL_DIR" ]; then
  DEFAULT_BIN="${HOME}/.local/bin"
  if [ "$NON_INTERACTIVE" = true ]; then
    INSTALL_DIR="$DEFAULT_BIN"
  else
    echo ""
    echo "[4/5] Specify installation directory on your machine for 'clibm':"
    read -r -p "Install location [default: ${DEFAULT_BIN}]: " input_dir
    INSTALL_DIR="${input_dir:-$DEFAULT_BIN}"
  fi
fi

INSTALL_DIR=$(clibm_normalize_dir "$INSTALL_DIR")

# 5. User PATH Registration
MODIFY_PATH=true
if [ "$NO_MODIFY_PATH" = true ]; then
  MODIFY_PATH=false
elif [ "$NON_INTERACTIVE" = false ]; then
  echo ""
  read -r -p "[5/5] Add installation directory (${INSTALL_DIR}) to user PATH? [Y/n]: " choice_path
  case "${choice_path:-Y}" in
    n|N|no|NO|false|2) MODIFY_PATH=false ;;
    *) MODIFY_PATH=true ;;
  esac
fi

mkdir -p "$INSTALL_DIR"

echo "==> Building Pure CLIBM Core & CLI (Release Mode)..."
cargo build --release

cp "target/release/clibm" "${INSTALL_DIR}/clibm"
chmod +x "${INSTALL_DIR}/clibm"
echo "==> Installed pure clibm binary to ${INSTALL_DIR}/clibm"

PATH_STATUS="skipped"
PATH_ENTRY_ADDED="false"
if [ "$MODIFY_PATH" = true ]; then
  clibm_path_add "core" "$INSTALL_DIR"
  PATH_STATUS="$CLIBM_LAST_PATH_STATUS"
  PATH_ENTRY_ADDED="$CLIBM_LAST_PATH_ADDED"
fi

echo ""
echo "--------------------------------------------------------------------"
echo " Installation Summary (Encoding: UTF-8-BOM EF BB BF):"
echo "   Profile        : $PROFILE"
echo "   Language       : $LANGUAGE"
echo "   Auto-Commit    : $AUTO_COMMIT"
echo "   Install Path   : $INSTALL_DIR"
echo "   User PATH      : $PATH_STATUS"
echo "--------------------------------------------------------------------"

# Persist global installation defaults in ~/.clibm/install-config.json (UTF-8-BOM)
GLOBAL_CFG_DIR="${HOME}/.clibm"
mkdir -p "${GLOBAL_CFG_DIR}/plugins"

printf '\xEF\xBB\xBF{\n  "install_profile": "%s",\n  "language": "%s",\n  "auto_commit": %s,\n  "install_path": "%s",\n  "path_entry_added": %s,\n  "mcp_transport": "both"\n}\n' \
  "$PROFILE" "$LANGUAGE" "$AUTO_COMMIT" "$INSTALL_DIR" "$PATH_ENTRY_ADDED" > "${GLOBAL_CFG_DIR}/install-config.json"

if [ "$PROFILE" = "with-defaults" ]; then
  echo "==> Installing default standalone plugins from ./plugins/..."
  for def_plugin in clibm-dep-audit clibm-export-md clibm-workflow-lint; do
    if [ -d "plugins/${def_plugin}" ]; then
      echo "   -> Building and installing plugins/${def_plugin}..."
      if [ "$MODIFY_PATH" = true ]; then
        (cd "plugins/${def_plugin}" && INSTALL_DIR="$INSTALL_DIR" ./install.sh -y)
      else
        (cd "plugins/${def_plugin}" && INSTALL_DIR="$INSTALL_DIR" ./install.sh -y --no-modify-path)
      fi
    fi
  done
else
  # Pure core: reset global plugin registry to empty list (UTF-8-BOM)
  printf '\xEF\xBB\xBF[]\n' > "${GLOBAL_CFG_DIR}/plugins/registry.json"
  echo "==> Pure Core active (0 pre-installed plugins)."
  echo "    Install any plugin later from its folder: cd plugins/<name> && ./install.sh"
fi

echo "===================================================================="
echo " CLIBM v0.1.0 Installation Complete!"
echo " Binary Location : ${INSTALL_DIR}/clibm"
echo " User PATH       : ${PATH_STATUS}"
if [ "$MODIFY_PATH" = true ]; then
  echo " Open a new terminal (or reload your shell profile) to use 'clibm'."
else
  echo " Note: User PATH was not modified (--no-modify-path)."
fi
echo "===================================================================="
