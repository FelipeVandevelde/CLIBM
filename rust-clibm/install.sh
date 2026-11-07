#!/usr/bin/env bash
# ==============================================================================
# CLIBM v0.1.0 — Interactive Core Installer (Linux / macOS)
# Encoding: UTF-8 with BOM (EF BB BF) for cross-platform compatibility
# Supports BOTH:
#   - Remote / Raw GitHub Install:
#     curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash
#   - Local Repository Install:
#     cd rust-clibm && ./install.sh
# ==============================================================================
# Presents interactive configuration options during installation:
#   1. Core Mode: Pure Core (without plugins) vs. Core + Default Plugins
#      (clibm-dep-audit, clibm-export-md, clibm-workflow-lint)
#   2. Language Selection: en (English), pt-BR (Português), es (Español)
#   3. Automatic Per-Modification Commits: true (default) or false
#   4. Machine Storage / Install Location: custom path (default: ~/.local/bin)
#   5. User PATH Registration: add install directory to user PATH (or --no-modify-path)
#
# Also supports non-interactive flags:
#   ./install.sh --pure|--with-defaults --lang en|pt-BR|es --auto-commit true|false --install-dir <PATH> [--no-modify-path]

set -euo pipefail

if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  cd "$SCRIPT_DIR"
else
  SCRIPT_DIR="$(pwd)"
fi

PROFILE=""
LANGUAGE=""
AUTO_COMMIT=""
INSTALL_DIR=""
RAW_BASE_URL=""
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
    --raw-base)
      RAW_BASE_URL="$2"
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

# >>> clibm-env-path (generated) >>>
clibm_init_env_path_helper() {
  local lib_dir="${HOME}/.clibm/lib"
  local helper_file="${lib_dir}/env-path.sh"
  local embedded_version=2
  local needs_write=1
  local bom=$'\xef\xbb\xbf'
  if [ -f "$helper_file" ]; then
    local first_three
    first_three=$(head -c 3 "$helper_file" 2>/dev/null || true)
    local existing_ver
    existing_ver=$(tr -d '\r' < "$helper_file" | sed "1s/^${bom}//" | sed -n '1,5s/^# clibm-env-path-version:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n1)
    if [ "$first_three" != "$bom" ] && [ -n "$existing_ver" ] && [ "$existing_ver" -ge "$embedded_version" ] 2>/dev/null; then
      needs_write=0
    fi
  fi
  if [ "$needs_write" -eq 1 ]; then
    mkdir -p "$lib_dir"
    cat <<'CLIBM_ENV_PATH_EOF' | tr -d '\r' > "$helper_file"
# clibm-env-path-version: 2
# ==============================================================================
# CLIBM Shared User PATH Helper (Linux / macOS — Bash 3.2+)
# Encoding: UTF-8 with BOM (EF BB BF) in repository; LF-clean when sourced
# Manages user-scope PATH marker blocks in shell profiles and reference-counted
# ownership in ~/.clibm/path-ledger without touching system-wide profiles.
# ==============================================================================

CLIBM_LAST_PATH_STATUS=""
CLIBM_LAST_PATH_DIR=""
CLIBM_LAST_PATH_ADDED="false"
CLIBM_REMOVED_PATH_COUNT=0
CLIBM_REMOVED_PATH_DIRS=""

clibm_normalize_dir() {
  local d="$1"
  d="${d#\"}"
  d="${d%\"}"
  case "$d" in
    "~") d="$HOME" ;;
    "~/"*) d="${HOME}/${d#\~/}" ;;
  esac
  case "$d" in
    /*) ;;
    *) d="$(pwd)/${d#./}" ;;
  esac
  while [ "$d" != "/" ] && [ "${d%/}" != "$d" ]; do
    d="${d%/}"
  done
  printf '%s\n' "$d"
}

clibm_sed_escape() {
  printf '%s\n' "$1" | sed -e 's/\\/\\\\/g' -e 's/\//\\\//g' -e 's/\./\\./g' -e 's/\[/\\[/g' -e 's/\]/\\]/g' -e 's/\*/\\*/g' -e 's/\^/\\^/g' -e 's/\$/\\$/g'
}

clibm_profile_has_marker() {
  local norm_dir="$1"
  local marker="# >>> clibm PATH: ${norm_dir} >>>"
  local rc
  for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
    if [ -f "$rc" ] && grep -Fq "$marker" "$rc" 2>/dev/null; then
      return 0
    fi
  done
  return 1
}

clibm_on_path() {
  local norm_dir
  norm_dir=$(clibm_normalize_dir "$1")
  local IFS=':'
  local entry clean_entry
  for entry in $PATH; do
    [ -z "$entry" ] && continue
    clean_entry="$entry"
    while [ "$clean_entry" != "/" ] && [ "${clean_entry%/}" != "$clean_entry" ]; do
      clean_entry="${clean_entry%/}"
    done
    if [ "$clean_entry" = "$norm_dir" ]; then
      return 0
    fi
  done
  if clibm_profile_has_marker "$norm_dir"; then
    return 0
  fi
  return 1
}

clibm_write_profile_block() {
  local norm_dir="$1"
  local marker_start="# >>> clibm PATH: ${norm_dir} >>>"
  local marker_end="# <<< clibm PATH: ${norm_dir} <<<"
  local found_any=0
  local rc
  for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
    if [ -f "$rc" ]; then
      found_any=1
    fi
  done
  if [ "$found_any" -eq 0 ]; then
    : > "${HOME}/.profile"
  fi
  for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
    if [ -f "$rc" ]; then
      if ! grep -Fq "$marker_start" "$rc" 2>/dev/null; then
        if [ -s "$rc" ] && [ -n "$(tail -c 1 "$rc" 2>/dev/null || true)" ]; then
          printf '\n' >> "$rc"
        fi
        {
          printf '%s\n' "$marker_start"
          printf 'export PATH="%s:$PATH"\n' "$norm_dir"
          printf '%s\n' "$marker_end"
        } >> "$rc"
      fi
    fi
  done
}

clibm_remove_profile_block() {
  local norm_dir="$1"
  local marker_start="# >>> clibm PATH: ${norm_dir} >>>"
  local esc_dir
  esc_dir=$(clibm_sed_escape "$norm_dir")
  local removed_any=1
  local rc
  for rc in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
    if [ -f "$rc" ] && grep -Fq "$marker_start" "$rc" 2>/dev/null; then
      sed -i.bak "/^# >>> clibm PATH: ${esc_dir} >>>\$/,/^# <<< clibm PATH: ${esc_dir} <<<\$/d" "$rc"
      rm -f "${rc}.bak"
      removed_any=0
    fi
  done
  return "$removed_any"
}

clibm_remove_dir_from_session_path() {
  local norm_dir="$1"
  local new_path=""
  local IFS=':'
  local entry clean_entry
  for entry in $PATH; do
    [ -z "$entry" ] && continue
    clean_entry="$entry"
    while [ "$clean_entry" != "/" ] && [ "${clean_entry%/}" != "$clean_entry" ]; do
      clean_entry="${clean_entry%/}"
    done
    if [ "$clean_entry" != "$norm_dir" ]; then
      if [ -z "$new_path" ]; then
        new_path="$entry"
      else
        new_path="${new_path}:${entry}"
      fi
    fi
  done
  export PATH="$new_path"
}

clibm_ledger_file() {
  printf '%s\n' "${HOME}/.clibm/path-ledger"
}

clibm_read_ledger() {
  local lf
  lf=$(clibm_ledger_file)
  if [ ! -f "$lf" ]; then
    return 0
  fi
  local bom=$'\xef\xbb\xbf'
  tr -d '\r' < "$lf" | sed "1s/^${bom}//"
}

clibm_path_add() {
  local owner="$1"
  local raw_dir="$2"
  local norm_dir
  norm_dir=$(clibm_normalize_dir "$raw_dir")
  local lf
  lf=$(clibm_ledger_file)
  mkdir -p "$(dirname "$lf")"

  local current_ledger
  current_ledger=$(clibm_read_ledger)

  local existing_added=0
  local l_owner l_dir l_added
  if [ -n "$current_ledger" ]; then
    while IFS=$'\t' read -r l_owner l_dir l_added; do
      [ -z "$l_owner" ] && continue
      if [ "$l_dir" = "$norm_dir" ] && [ "$l_added" = "1" ]; then
        existing_added=1
        break
      fi
    done <<EOF
$current_ledger
EOF
  fi

  local added_by_clibm=0
  local status="already present"
  if ! clibm_on_path "$norm_dir"; then
    clibm_write_profile_block "$norm_dir"
    export PATH="${norm_dir}:${PATH}"
    added_by_clibm=1
    status="added"
  else
    added_by_clibm="$existing_added"
    if [ "$added_by_clibm" -eq 1 ]; then
      clibm_write_profile_block "$norm_dir"
    fi
    case ":${PATH}:" in
      *":${norm_dir}:"*) ;;
      *) export PATH="${norm_dir}:${PATH}" ;;
    esac
    status="already present"
  fi

  local tmp_ledger="${lf}.tmp.$$"
  : > "$tmp_ledger"
  local old_dirs=""
  if [ -n "$current_ledger" ]; then
    while IFS=$'\t' read -r l_owner l_dir l_added; do
      [ -z "$l_owner" ] && continue
      if [ "$l_owner" = "$owner" ]; then
        if [ "$l_dir" != "$norm_dir" ] && [ "$l_added" = "1" ]; then
          old_dirs="${old_dirs}${l_dir}"$'\n'
        fi
      else
        if [ "$l_dir" = "$norm_dir" ] && [ "$added_by_clibm" -eq 1 ]; then
          l_added=1
        fi
        printf '%s\t%s\t%s\n' "$l_owner" "$l_dir" "$l_added" >> "$tmp_ledger"
      fi
    done <<EOF
$current_ledger
EOF
  fi
  printf '%s\t%s\t%s\n' "$owner" "$norm_dir" "$added_by_clibm" >> "$tmp_ledger"
  mv -f "$tmp_ledger" "$lf"

  if [ -n "$old_dirs" ]; then
    local updated_ledger
    updated_ledger=$(clibm_read_ledger)
    local od still_owned
    while IFS= read -r od; do
      [ -z "$od" ] && continue
      still_owned=0
      while IFS=$'\t' read -r l_owner l_dir l_added; do
        if [ "$l_dir" = "$od" ]; then
          still_owned=1
          break
        fi
      done <<EOF
$updated_ledger
EOF
      if [ "$still_owned" -eq 0 ]; then
        clibm_remove_profile_block "$od" || true
        clibm_remove_dir_from_session_path "$od"
      fi
    done <<EOF
$old_dirs
EOF
  fi

  CLIBM_LAST_PATH_STATUS="$status"
  CLIBM_LAST_PATH_DIR="$norm_dir"
  if [ "$added_by_clibm" -eq 1 ]; then
    CLIBM_LAST_PATH_ADDED="true"
  else
    CLIBM_LAST_PATH_ADDED="false"
  fi
}

clibm_path_owner_dirs() {
  local owner="$1"
  local current_ledger
  current_ledger=$(clibm_read_ledger)
  [ -z "$current_ledger" ] && return 0
  local l_owner l_dir l_added
  while IFS=$'\t' read -r l_owner l_dir l_added; do
    [ -z "$l_owner" ] && continue
    if [ "$l_owner" = "$owner" ]; then
      printf '%s\n' "$l_dir"
    fi
  done <<EOF
$current_ledger
EOF
}

clibm_path_all_dirs() {
  local current_ledger
  current_ledger=$(clibm_read_ledger)
  [ -z "$current_ledger" ] && return 0
  local l_owner l_dir l_added
  while IFS=$'\t' read -r l_owner l_dir l_added; do
    [ -z "$l_owner" ] && continue
    printf '%s\n' "$l_dir"
  done <<EOF
$current_ledger
EOF
}

clibm_path_remove() {
  local owner="$1"
  CLIBM_REMOVED_PATH_COUNT=0
  CLIBM_REMOVED_PATH_DIRS=""
  local lf
  lf=$(clibm_ledger_file)
  if [ ! -f "$lf" ]; then
    return 0
  fi

  local current_data
  current_data=$(clibm_read_ledger)
  [ -z "$current_data" ] && return 0

  local tmp_keep="${lf}.tmp.$$"
  : > "$tmp_keep"
  local owner_added_dirs=""
  local keep_count=0
  local l_owner l_dir l_added

  while IFS=$'\t' read -r l_owner l_dir l_added; do
    [ -z "$l_owner" ] && continue
    if [ "$l_owner" = "$owner" ]; then
      if [ "$l_added" = "1" ]; then
        owner_added_dirs="${owner_added_dirs}${l_dir}"$'\n'
      fi
    else
      printf '%s\t%s\t%s\n' "$l_owner" "$l_dir" "$l_added" >> "$tmp_keep"
      keep_count=$((keep_count + 1))
    fi
  done <<EOF
$current_data
EOF

  if [ "$keep_count" -eq 0 ]; then
    rm -f "$tmp_keep" "$lf"
  else
    mv -f "$tmp_keep" "$lf"
  fi

  if [ -n "$owner_added_dirs" ]; then
    local cand still_ref
    local remaining_data
    remaining_data=$(clibm_read_ledger)
    while IFS= read -r cand; do
      [ -z "$cand" ] && continue
      still_ref=0
      if [ -n "$remaining_data" ]; then
        while IFS=$'\t' read -r l_owner l_dir l_added; do
          if [ "$l_dir" = "$cand" ]; then
            still_ref=1
            break
          fi
        done <<EOF
$remaining_data
EOF
      fi
      if [ "$still_ref" -eq 0 ]; then
        if clibm_remove_profile_block "$cand"; then
          CLIBM_REMOVED_PATH_COUNT=$((CLIBM_REMOVED_PATH_COUNT + 1))
          CLIBM_REMOVED_PATH_DIRS="${CLIBM_REMOVED_PATH_DIRS}${cand}"$'\n'
        fi
        clibm_remove_dir_from_session_path "$cand"
      fi
    done <<EOF
$(printf '%s' "$owner_added_dirs" | awk '!seen[$0]++')
EOF
  fi
}

clibm_path_remove_all() {
  CLIBM_REMOVED_PATH_COUNT=0
  CLIBM_REMOVED_PATH_DIRS=""
  local lf
  lf=$(clibm_ledger_file)
  if [ ! -f "$lf" ]; then
    return 0
  fi

  local current_data
  current_data=$(clibm_read_ledger)
  rm -f "$lf"
  [ -z "$current_data" ] && return 0

  local added_dirs=""
  local l_owner l_dir l_added
  while IFS=$'\t' read -r l_owner l_dir l_added; do
    [ -z "$l_owner" ] && continue
    if [ "$l_added" = "1" ]; then
      added_dirs="${added_dirs}${l_dir}"$'\n'
    fi
  done <<EOF
$current_data
EOF

  if [ -n "$added_dirs" ]; then
    local cand
    while IFS= read -r cand; do
      [ -z "$cand" ] && continue
      if clibm_remove_profile_block "$cand"; then
        CLIBM_REMOVED_PATH_COUNT=$((CLIBM_REMOVED_PATH_COUNT + 1))
        CLIBM_REMOVED_PATH_DIRS="${CLIBM_REMOVED_PATH_DIRS}${cand}"$'\n'
      fi
      clibm_remove_dir_from_session_path "$cand"
    done <<EOF
$(printf '%s' "$added_dirs" | awk '!seen[$0]++')
EOF
  fi
}

clibm_core_installed() {
  local extra_dir="${1:-}"
  if [ -f "${HOME}/.clibm/install-config.json" ]; then
    return 0
  fi
  if command -v clibm >/dev/null 2>&1; then
    return 0
  fi
  local d
  for d in "$extra_dir" "${HOME}/.local/bin" "${HOME}/.clibm-bin" "${HOME}/.cargo/bin"; do
    if [ -n "$d" ] && [ -x "${d}/clibm" ]; then
      return 0
    fi
  done
  return 1
}

clibm_raw_repo_base() {
  local base="${CLIBM_RAW_BASE:-https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm}"
  printf '%s\n' "${base%/}"
}

clibm_prompt() {
  local prompt_text="$1"
  local __outvar="$2"
  local ans=""
  if [ -t 0 ]; then
    read -r -p "$prompt_text" ans || true
  elif [ -e /dev/tty ] && [ -r /dev/tty ]; then
    read -r -p "$prompt_text" ans < /dev/tty || true
  fi
  printf -v "$__outvar" '%s' "$ans"
}

clibm_download_url() {
  local url="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$dest"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$dest" "$url"
  else
    echo "ERROR: Neither 'curl' nor 'wget' is available to download ${url}" >&2
    return 1
  fi
}

clibm_resolve_local_source_dir() {
  local crate_name="$1"
  local remote_subpath="${2:-}"
  local candidate_dir="${3:-}"
  local cwd_dir
  cwd_dir="$(pwd)"
  local d
  for d in \
    "$candidate_dir" \
    "${candidate_dir:+${candidate_dir}/${remote_subpath}}" \
    "${candidate_dir:+${candidate_dir}/rust-clibm/${remote_subpath}}" \
    "${candidate_dir:+${candidate_dir}/rust-clibm}" \
    "$cwd_dir" \
    "${cwd_dir}/${remote_subpath}" \
    "${cwd_dir}/rust-clibm/${remote_subpath}" \
    "${cwd_dir}/rust-clibm"; do
    [ -z "$d" ] && continue
    d="${d%/}"
    if [ -f "${d}/Cargo.toml" ] && [ -f "${d}/src/main.rs" ]; then
      if grep -Eq "^[[:space:]]*name[[:space:]]*=[[:space:]]*\"${crate_name}\"" "${d}/Cargo.toml" 2>/dev/null; then
        printf '%s\n' "$d"
        return 0
      fi
    fi
  done
  return 1
}

clibm_build_and_stage() {
  local crate_name="$1"
  local remote_subpath="$2"
  local dest_binary="$3"
  local local_candidate_dir="$4"
  local raw_base_override="$5"
  shift 5

  mkdir -p "$(dirname "$dest_binary")"

  local local_dir=""
  if local_dir=$(clibm_resolve_local_source_dir "$crate_name" "$remote_subpath" "$local_candidate_dir"); then
    echo "==> [Local Mode] Building ${crate_name} from local source (${local_dir})..."
    (cd "$local_dir" && cargo build --release)
    cp "${local_dir}/target/release/${crate_name}" "$dest_binary"
    chmod +x "$dest_binary"
    return 0
  fi

  local base_url
  if [ -n "$raw_base_override" ]; then
    base_url="${raw_base_override%/}"
  else
    local root_base
    root_base=$(clibm_raw_repo_base)
    if [ -n "$remote_subpath" ]; then
      base_url="${root_base}/${remote_subpath#/}"
    else
      base_url="$root_base"
    fi
  fi

  local tmp_build_dir
  tmp_build_dir=$(mktemp -d "${TMPDIR:-/tmp}/${crate_name}-build.XXXXXX")
  echo "==> [Remote Mode] Fetching ${crate_name} source files from GitHub (${base_url})..."
  local rel_file clean_rel
  for rel_file in "$@"; do
    clean_rel="${rel_file#/}"
    if ! clibm_download_url "${base_url}/${clean_rel}" "${tmp_build_dir}/${clean_rel}"; then
      rm -rf "$tmp_build_dir"
      return 1
    fi
  done

  echo "==> Building ${crate_name} (Release Mode)..."
  if ! (cd "$tmp_build_dir" && cargo build --release); then
    rm -rf "$tmp_build_dir"
    return 1
  fi
  cp "${tmp_build_dir}/target/release/${crate_name}" "$dest_binary"
  chmod +x "$dest_binary"
  rm -rf "$tmp_build_dir"
  return 0
}

clibm_register_global_plugin() {
  local name="$1"
  local description="$2"
  local hooks_csv="$3"
  local executable="$4"
  local version="${5:-0.1.0}"
  local plugins_dir="${HOME}/.clibm/plugins"
  local reg_file="${plugins_dir}/registry.json"
  mkdir -p "$plugins_dir"

  local hooks_json=""
  local IFS=','
  local h
  for h in $hooks_csv; do
    [ -z "$h" ] && continue
    if [ -z "$hooks_json" ]; then
      hooks_json="\"${h}\""
    else
      hooks_json="${hooks_json}, \"${h}\""
    fi
  done
  if [ -z "$hooks_json" ]; then
    hooks_json="\"manual-run\""
  fi

  local new_obj
  new_obj=$(printf '{\n    "name": "%s",\n    "version": "%s",\n    "description": "%s",\n    "enabled": true,\n    "hooks": [%s],\n    "executable": "%s"\n  }' \
    "$name" "$version" "$description" "$hooks_json" "$executable")

  if [ ! -f "$reg_file" ] || ! grep -Fq "\"name\"" "$reg_file" 2>/dev/null; then
    printf '\xEF\xBB\xBF[\n  %s\n]\n' "$new_obj" > "$reg_file"
  elif ! grep -Fq "\"name\": \"${name}\"" "$reg_file" 2>/dev/null; then
    local bom=$'\xef\xbb\xbf'
    local body
    body=$(tr -d '\r' < "$reg_file" | sed "1s/^${bom}//" | sed '$d')
    printf '\xEF\xBB\xBF%s,\n  %s\n]\n' "$body" "$new_obj" > "$reg_file"
  fi
}
CLIBM_ENV_PATH_EOF
  fi
  # shellcheck disable=SC1090
  . "$helper_file"
}
clibm_init_env_path_helper
# <<< clibm-env-path (generated) <<<

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
    echo "  2) Core + Default Plugins (builds dep-audit, export-md, workflow-lint)"
    clibm_prompt "Select option [1-2, default: 1]: " choice_profile
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
    clibm_prompt "Select language [1-3, default: 1]: " choice_lang
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
    clibm_prompt "Select option [1-2, default: 1]: " choice_auto
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
    clibm_prompt "Install location [default: ${DEFAULT_BIN}]: " input_dir
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
  clibm_prompt "[5/5] Add installation directory (${INSTALL_DIR}) to user PATH? [Y/n]: " choice_path
  case "${choice_path:-Y}" in
    n|N|no|NO|false|2) MODIFY_PATH=false ;;
    *) MODIFY_PATH=true ;;
  esac
fi

mkdir -p "$INSTALL_DIR"

clibm_build_and_stage \
  "clibm" \
  "" \
  "${INSTALL_DIR}/clibm" \
  "$SCRIPT_DIR" \
  "$RAW_BASE_URL" \
  "Cargo.toml" \
  "scripts/env-path.ps1" \
  "scripts/env-path.sh" \
  "src/main.rs" \
  "src/lib.rs" \
  "src/cli.rs" \
  "src/canonical.rs" \
  "src/cas.rs" \
  "src/models.rs" \
  "src/index.rs" \
  "src/versioning.rs" \
  "src/plugin.rs" \
  "src/engine.rs"

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
  echo "==> Installing default standalone plugins (clibm-dep-audit, clibm-export-md, clibm-workflow-lint)..."
  EFFECTIVE_RAW_BASE="${RAW_BASE_URL:-$(clibm_raw_repo_base)}"
  EFFECTIVE_RAW_BASE="${EFFECTIVE_RAW_BASE%/}"
  for def_plugin in clibm-dep-audit clibm-export-md clibm-workflow-lint; do
    LOCAL_PLUGIN_DIR=""
    if LOCAL_PLUGIN_DIR=$(clibm_resolve_local_source_dir "$def_plugin" "plugins/${def_plugin}" "$SCRIPT_DIR") && [ -f "${LOCAL_PLUGIN_DIR}/install.sh" ]; then
      echo "   -> [Local Mode] Building and installing ${def_plugin}..."
      if [ "$MODIFY_PATH" = true ]; then
        (cd "$LOCAL_PLUGIN_DIR" && INSTALL_DIR="$INSTALL_DIR" ./install.sh -y)
      else
        (cd "$LOCAL_PLUGIN_DIR" && INSTALL_DIR="$INSTALL_DIR" ./install.sh -y --no-modify-path)
      fi
    else
      echo "   -> [Remote Mode] Fetching and installing ${def_plugin} from GitHub..."
      TMP_PLUGIN_SH=$(mktemp "${TMPDIR:-/tmp}/${def_plugin}-install.XXXXXX.sh")
      if clibm_download_url "${EFFECTIVE_RAW_BASE}/plugins/${def_plugin}/install.sh" "$TMP_PLUGIN_SH"; then
        if [ "$MODIFY_PATH" = true ]; then
          INSTALL_DIR="$INSTALL_DIR" CLIBM_RAW_BASE="$EFFECTIVE_RAW_BASE" bash "$TMP_PLUGIN_SH" -y
        else
          INSTALL_DIR="$INSTALL_DIR" CLIBM_RAW_BASE="$EFFECTIVE_RAW_BASE" bash "$TMP_PLUGIN_SH" -y --no-modify-path
        fi
      fi
      rm -f "$TMP_PLUGIN_SH"
    fi
  done
else
  # Pure core: reset global plugin registry to empty list (UTF-8-BOM)
  printf '\xEF\xBB\xBF[]\n' > "${GLOBAL_CFG_DIR}/plugins/registry.json"
  echo "==> Pure Core active (0 pre-installed plugins)."
  echo "    Install any plugin later via its folder or raw GitHub URL."
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
