#!/usr/bin/env bash
# ==============================================================================
# CLIBM v0.1.0 -- Interactive Core Installer (Linux / macOS)
# Encoding: UTF-8 (without BOM) for cross-platform compatibility
# Supports BOTH:
#   - Remote / Raw GitHub Install:
#     curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash
#   - Local Repository Install:
#     cd rust-clibm && ./install.sh
# ==============================================================================
# Presents interactive configuration options during installation:
#   1. Core Mode: Pure Core (without plugins) vs. Core + Default Plugins
#      (clibm-dep-audit, clibm-export-md, clibm-workflow-lint)
#   2. Language Selection: en (English), pt-BR (Portugues), es (Espanol)
#   3. Automatic Per-Modification Commits: true (default) or false
#   4. Machine Storage / Install Location: custom path (default: ~/.local/bin)
#   5. User PATH Registration: add install directory to user PATH (or --no-modify-path)
#
# Also supports non-interactive flags:
#   ./install.sh --pure|--with-defaults --lang en|pt-BR|es --auto-commit true|false --install-dir <PATH> [--no-modify-path]

set -euo pipefail

CALLER_DIR="$(pwd)"
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  cd "$SCRIPT_DIR"
else
  SCRIPT_DIR="$CALLER_DIR"
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
  local home_dir="${CLIBM_HOME:-${HOME}}"
  local lib_dir="${home_dir%/}/.clibm/lib"
  local helper_file="${lib_dir}/env-path.sh"
  local embedded_version=6
  local needs_write=1
  local bom=$'\xef\xbb\xbf'
  if [ -f "$helper_file" ]; then
    local first_three
    first_three=$(head -c 3 "$helper_file" 2>/dev/null || true)
    local existing_ver
    existing_ver=$(tr -d '\r' < "$helper_file" | sed "1s/^${bom}//" | sed -n '1,5s/^# clibm-env-path-version:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | head -n1)
    if [ "$first_three" != "$bom" ] && [ -n "$existing_ver" ] && [ "$existing_ver" -gt "$embedded_version" ] 2>/dev/null; then
      needs_write=0
    fi
  fi
  if [ "$needs_write" -eq 1 ]; then
    mkdir -p "$lib_dir"
    cat <<'CLIBM_ENV_PATH_EOF' | tr -d '\r' > "$helper_file"
# clibm-env-path-version: 6
# ==============================================================================
# CLIBM Shared User PATH Helper (Linux / macOS -- Bash 3.2+)
# Encoding: UTF-8 (without BOM) in repository; LF-clean when sourced
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

CLIBM_LAST_BUILD_COMMIT=""

clibm_git_source_url() {
  printf '%s\n' "${CLIBM_GIT_URL:-https://github.com/FelipeVandevelde/CLIBM.git}"
}

clibm_git_source_ref() {
  printf '%s\n' "${CLIBM_GIT_REF:-main}"
}

# Copies a freshly built binary over the installed one (atomic rename, so a running
# copy keeps working until it exits).
clibm_install_binary() {
  local src="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  cp "$src" "${dest}.new.$$"
  chmod +x "${dest}.new.$$"
  mv -f "${dest}.new.$$" "$dest"
}

# Builds a CLIBM crate and installs its binary at <dest_binary>.
# - Local mode (a clone of the repository is found): `cargo build --release -p <crate>`.
# - Remote mode: `cargo install --git <CLIBM_GIT_URL> --branch <CLIBM_GIT_REF> --root <tmp>`,
#   which builds the whole Cargo workspace correctly. Set CLIBM_FORCE_REMOTE=1 to skip
#   local mode. Extra cargo flags come from CARGO_BUILD_FEATURES. The raw-base argument
#   and source file list are accepted for older callers but unused.
# Sets CLIBM_LAST_BUILD_COMMIT to the commit that was built (when known).
clibm_build_and_stage() {
  local crate_name="$1"
  local remote_subpath="$2"
  local dest_binary="$3"
  local local_candidate_dir="$4"
  shift 4
  if [ $# -gt 0 ]; then shift; fi

  CLIBM_LAST_BUILD_COMMIT=""
  if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: cargo was not found. Install Rust from https://rustup.rs, open a new terminal and run the installer again." >&2
    return 1
  fi

  local local_dir=""
  if [ -z "${CLIBM_FORCE_REMOTE:-}" ] && local_dir=$(clibm_resolve_local_source_dir "$crate_name" "$remote_subpath" "$local_candidate_dir"); then
    echo "==> [Local Mode] Building ${crate_name} from local source (${local_dir})..."
    # shellcheck disable=SC2086
    if ! (cd "$local_dir" && cargo build --release -p "$crate_name" ${CARGO_BUILD_FEATURES:-}); then
      echo "ERROR: cargo build failed for ${crate_name}" >&2
      return 1
    fi
    local target_dir
    target_dir=$(cd "$local_dir" && cargo metadata --no-deps --format-version 1 2>/dev/null | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' || true)
    if [ -z "$target_dir" ]; then
      target_dir="${local_dir}/target"
    fi
    if [ ! -f "${target_dir}/release/${crate_name}" ]; then
      echo "ERROR: compiled binary not found at ${target_dir}/release/${crate_name}" >&2
      return 1
    fi
    clibm_install_binary "${target_dir}/release/${crate_name}" "$dest_binary"
    CLIBM_LAST_BUILD_COMMIT=$(git -C "$local_dir" rev-parse HEAD 2>/dev/null || true)
    return 0
  fi

  local url ref tmp_root
  url=$(clibm_git_source_url)
  ref=$(clibm_git_source_ref)
  tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/${crate_name}-build.XXXXXX")
  echo "==> [Remote Mode] Building ${crate_name} from ${url} (branch ${ref})..."
  # shellcheck disable=SC2086
  # With <tmp>/bin on PATH, cargo does not ask the user to add the temp folder to PATH.
  if ! PATH="${tmp_root}/bin:${PATH}" cargo install --git "$url" --branch "$ref" --root "$tmp_root" --locked --force "$crate_name" ${CARGO_BUILD_FEATURES:-}; then
    rm -rf "$tmp_root"
    echo "ERROR: cargo install failed for ${crate_name}" >&2
    return 1
  fi
  if [ ! -f "${tmp_root}/bin/${crate_name}" ]; then
    rm -rf "$tmp_root"
    echo "ERROR: compiled binary not found for ${crate_name}" >&2
    return 1
  fi
  clibm_install_binary "${tmp_root}/bin/${crate_name}" "$dest_binary"
  if [ -f "${tmp_root}/.crates2.json" ]; then
    CLIBM_LAST_BUILD_COMMIT=$(grep -o '#[0-9a-f]\{40\}' "${tmp_root}/.crates2.json" | awk 'NR == 1 { print substr($0, 2) }' || true)
  fi
  rm -rf "$tmp_root"
  return 0
}

clibm_find_project_plugin_dirs() {
  local home_dir="${CLIBM_HOME:-${HOME}}"
  local global_clibm="${home_dir%/}/.clibm"
  local user_clibm="${HOME%/}/.clibm"
  local seen_list=""
  local root curr start_curr parent base dot_clibm
  for root in "${CLIBM_PROJECT_ROOT:-}" "$@" "$(pwd)"; do
    [ -z "$root" ] && continue
    curr=$(clibm_normalize_dir "$root")
    start_curr="$curr"
    while [ -n "$curr" ]; do
      base=$(basename "$curr")
      parent=$(dirname "$curr")
      if [ "$base" = "plugins" ] && [ "$(basename "$parent")" = ".clibm" ]; then
        if [ "$parent" != "$global_clibm" ] && [ "$parent" != "$user_clibm" ]; then
          case "
${seen_list}
" in
            *"
${curr}
"*) ;;
            *)
              seen_list="${seen_list}
${curr}"
              printf '%s\n' "$curr"
              ;;
          esac
        fi
      elif [ "$base" = ".clibm" ]; then
        if [ "$curr" != "$global_clibm" ] && [ "$curr" != "$user_clibm" ]; then
          case "
${seen_list}
" in
            *"
${curr}/plugins
"*) ;;
            *)
              seen_list="${seen_list}
${curr}/plugins"
              printf '%s\n' "${curr}/plugins"
              ;;
          esac
        fi
      elif [ -d "${curr}/.clibm" ]; then
        dot_clibm="${curr}/.clibm"
        if [ "$dot_clibm" != "$global_clibm" ] && [ "$dot_clibm" != "$user_clibm" ]; then
          if [ "$curr" = "$start_curr" ] || [ -d "${dot_clibm}/objects" ] || [ -f "${dot_clibm}/HEAD" ] || [ -f "${dot_clibm}/config.json" ]; then
            case "
${seen_list}
" in
              *"
${dot_clibm}/plugins
"*) ;;
              *)
                seen_list="${seen_list}
${dot_clibm}/plugins"
                printf '%s\n' "${dot_clibm}/plugins"
                ;;
            esac
          fi
        fi
      fi
      if [ "$parent" = "$curr" ] || [ -z "$parent" ]; then
        break
      fi
      curr="$parent"
    done
  done
}

clibm_global_registry_file() {
  local home_dir="${CLIBM_HOME:-${HOME}}"
  printf '%s\n' "${home_dir%/}/.clibm/plugins/registry.json"
}

# Registry files are JSON arrays. The global registry (~/.clibm/plugins/registry.json)
# holds one manifest per installed plugin; a project's .clibm/plugins/registry.json only
# holds {"name": ..., "enabled": ...} overrides. Files are written with one compact
# object per line (UTF-8, no BOM), which keeps these helpers line-based.

# Prints every top-level object of a registry file on its own line (also converts the
# older multi-line format).
clibm_registry_lines() {
  local file="$1"
  local bom=$'\xef\xbb\xbf'
  if [ ! -f "$file" ]; then
    return 0
  fi
  tr -d '\r' < "$file" | sed "1s/^${bom}//" | awk '
    {
      n = length($0)
      for (i = 1; i <= n; i++) {
        c = substr($0, i, 1)
        if (depth > 0) buf = buf c
        if (esc) { esc = 0; continue }
        if (instr) {
          if (c == "\\") esc = 1
          else if (c == "\"") instr = 0
          continue
        }
        if (c == "\"") { instr = 1; continue }
        if (c == "{") {
          if (depth == 0) buf = "{"
          depth++
        } else if (c == "}") {
          depth--
          if (depth == 0) { print buf; buf = "" }
        }
      }
      if (depth > 0) buf = buf " "
    }'
}

# Prints the value of the first "<key>": "<string>" pair of a one-line JSON object.
clibm_json_string_field() {
  printf '%s\n' "$1" | awk -v key="$2" '
    {
      pat = "\"" key "\"[ \t]*:[ \t]*\"[^\"]*\""
      if (match($0, pat)) {
        s = substr($0, RSTART, RLENGTH)
        sub(/^"[^"]*"[ \t]*:[ \t]*"/, "", s)
        sub(/"$/, "", s)
        print s
      }
    }'
}

clibm_registry_entry_name() {
  clibm_json_string_field "$1" "name"
}

clibm_registry_names() {
  local line
  clibm_registry_lines "$1" | while IFS= read -r line; do
    clibm_registry_entry_name "$line"
  done
}

clibm_registry_has() {
  local names
  names=$(clibm_registry_names "$1" || true)
  printf '%s\n' "$names" | grep -Fxq -- "$2"
}

# Reads one-line objects on stdin and writes them as a registry file.
clibm_registry_write() {
  local file="$1"
  local tmp_file="${file}.tmp.$$"
  mkdir -p "$(dirname "$file")"
  awk '
    NF { lines[++n] = $0 }
    END {
      if (n == 0) { print "[]"; exit }
      print "["
      for (i = 1; i <= n; i++) printf "%s%s\n", lines[i], (i < n ? "," : "")
      print "]"
    }' > "$tmp_file"
  mv -f "$tmp_file" "$file"
}

# Prints the registry lines of <file> except the entry named <name>.
clibm_registry_without() {
  local file="$1"
  local name="$2"
  local line entry
  clibm_registry_lines "$file" | while IFS= read -r line; do
    entry=$(clibm_registry_entry_name "$line")
    if [ -n "$entry" ] && [ "$entry" != "$name" ]; then
      printf '%s\n' "$line"
    fi
  done
}

clibm_registry_upsert() {
  local file="$1"
  local name="$2"
  local obj="$3"
  {
    clibm_registry_without "$file" "$name"
    printf '%s\n' "$obj"
  } | clibm_registry_write "$file"
}

clibm_registry_remove() {
  local file="$1"
  local name="$2"
  clibm_registry_without "$file" "$name" | clibm_registry_write "$file"
}

clibm_json_escape() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}

# Asks a plugin binary for its registry entry (`<exe> --manifest --executable <exe>`).
clibm_plugin_manifest() {
  local exe="$1"
  local hooks="${2:-}"
  local out
  if [ ! -x "$exe" ]; then
    return 1
  fi
  if [ -n "$hooks" ]; then
    out=$("$exe" --manifest --executable "$exe" --hooks "$hooks" 2>/dev/null) || return 1
  else
    out=$("$exe" --manifest --executable "$exe" 2>/dev/null) || return 1
  fi
  out=$(printf '%s\n' "$out" | awk '/^[[:space:]]*\{/ { print; exit }')
  if [ -z "$out" ]; then
    return 1
  fi
  printf '%s\n' "$out"
}

# Sets (true/false) or removes ("remove") a plugin override in a project's registry.
clibm_set_project_override() {
  local plugins_dir="$1"
  local name="$2"
  local value="$3"
  local file="${plugins_dir%/}/registry.json"
  local line entry enabled
  if [ "$value" = "remove" ] && [ ! -f "$file" ]; then
    return 0
  fi
  {
    clibm_registry_lines "$file" | while IFS= read -r line; do
      entry=$(clibm_registry_entry_name "$line")
      if [ -z "$entry" ] || [ "$entry" = "$name" ]; then
        continue
      fi
      enabled=$(printf '%s\n' "$line" | awk 'match($0, /"enabled"[ \t]*:[ \t]*(true|false)/) { s = substr($0, RSTART, RLENGTH); sub(/.*:[ \t]*/, "", s); print s }')
      printf '{"name":"%s","enabled":%s}\n' "$entry" "${enabled:-true}"
    done
    if [ "$value" != "remove" ]; then
      printf '{"name":"%s","enabled":%s}\n' "$name" "$value"
    fi
  } | clibm_registry_write "$file"
}

# Registers an installed plugin in the global registry. The entry comes from the plugin
# itself (`--manifest`), so MCP tool declarations are kept; plugins without `--manifest`
# get a basic entry. Scope "project-only" installs the plugin disabled by default and
# enables it only for the current project.
clibm_register_global_plugin() {
  local name="$1"
  local description="$2"
  local hooks_csv="$3"
  local executable="$4"
  local version="${5:-0.1.0}"
  local scope="${6:-global}"
  local project_dir="${7:-}"
  local global_reg clean_hooks obj hooks_json base_proj pdir
  global_reg=$(clibm_global_registry_file)

  clean_hooks=$(printf '%s' "$hooks_csv" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | awk 'NF' | paste -sd, - || true)
  if [ -z "$clean_hooks" ]; then
    clean_hooks="manual-run"
  fi

  if ! obj=$(clibm_plugin_manifest "$executable" "$clean_hooks"); then
    hooks_json=$(printf '%s' "$clean_hooks" | tr ',' '\n' | awk 'NF { printf "%s\"%s\"", (n++ ? "," : ""), $0 }')
    obj=$(printf '{"name":"%s","version":"%s","description":"%s","enabled":true,"hooks":[%s],"executable":"%s"}' \
      "$name" "$version" "$(clibm_json_escape "$description")" "$hooks_json" "$(clibm_json_escape "$executable")")
  fi
  if [ "$scope" = "project-only" ]; then
    obj=$(printf '%s' "$obj" | sed 's/"enabled":[[:space:]]*true/"enabled":false/')
  fi

  clibm_registry_upsert "$global_reg" "$name" "$obj"
  echo "==> Registered ${name} in plugin registry: ${global_reg}"

  if [ "$scope" = "project-only" ]; then
    base_proj="${project_dir:-$(pwd)}"
    pdir=$(clibm_find_project_plugin_dirs "$base_proj" | awk 'NR == 1' || true)
    if [ -z "$pdir" ]; then
      pdir="${base_proj%/}/.clibm/plugins"
    fi
    clibm_set_project_override "$pdir" "$name" true
    echo "==> Enabled ${name} for this project only: ${pdir}/registry.json"
  fi
}

# Removes a plugin from the global registry, from the overrides of the projects found
# around <project_dir> / extra dirs, and from legacy registry.json files next to binaries.
clibm_unregister_plugin() {
  local name="$1"
  local project_dir="${2:-}"
  shift $(( $# >= 2 ? 2 : $# ))
  local global_reg rfile proj_dirs pdir extra_d
  global_reg=$(clibm_global_registry_file)

  for rfile in "$global_reg" "$@"; do
    if [ -z "$rfile" ]; then
      continue
    fi
    if [ "$rfile" != "$global_reg" ]; then
      rfile="${rfile%/}/registry.json"
    fi
    if [ -f "$rfile" ] && clibm_registry_has "$rfile" "$name"; then
      clibm_registry_remove "$rfile" "$name"
      echo "==> Removed ${name} from plugin registry: ${rfile}"
    fi
  done

  proj_dirs=$(clibm_find_project_plugin_dirs "$project_dir" "$@" || true)
  while IFS= read -r pdir; do
    if [ -z "$pdir" ]; then
      continue
    fi
    if [ -f "${pdir}/registry.json" ] && clibm_registry_has "${pdir}/registry.json" "$name"; then
      clibm_set_project_override "$pdir" "$name" remove
      echo "==> Removed ${name} from project plugin settings: ${pdir}/registry.json"
    fi
  done <<EOF
$proj_dirs
EOF
}

# Uninstalls a plugin: deletes its binary from every known install folder, unregisters
# it and releases its user PATH ownership.
clibm_uninstall_plugin() {
  local name="$1"
  local home_dir="${CLIBM_HOME:-${HOME}}"
  local owner_dirs exe_path exe_dir cfg_file cfg_dir search_dirs dir rdir line
  owner_dirs=$(clibm_path_owner_dirs "$name" || true)

  exe_dir=""
  exe_path=$(clibm_registry_lines "$(clibm_global_registry_file)" | while IFS= read -r line; do
    if [ "$(clibm_registry_entry_name "$line")" = "$name" ]; then
      clibm_json_string_field "$line" "executable"
    fi
  done || true)
  if [ -n "$exe_path" ]; then
    exe_dir=$(dirname "$exe_path")
  fi

  cfg_dir=""
  cfg_file="${home_dir%/}/.clibm/install-config.json"
  if [ -f "$cfg_file" ]; then
    cfg_dir=$(clibm_json_string_field "$(tr -d '\r\n' < "$cfg_file")" "install_path")
    if [ -n "$cfg_dir" ]; then
      cfg_dir=$(clibm_normalize_dir "$cfg_dir")
    fi
  fi

  search_dirs=$(
    {
      if [ -n "$owner_dirs" ]; then printf '%s\n' "$owner_dirs"; fi
      if [ -n "$exe_dir" ]; then printf '%s\n' "$exe_dir"; fi
      if [ -n "$cfg_dir" ]; then printf '%s\n' "$cfg_dir"; fi
      printf '%s\n' "${home_dir%/}/.local/bin" "${home_dir%/}/.cargo/bin" "${home_dir%/}/.clibm-bin"
    } | awk 'NF && !seen[$0]++'
  )

  local -a extra_dirs=()
  while IFS= read -r dir; do
    if [ -z "$dir" ]; then
      continue
    fi
    extra_dirs+=("$dir")
    if [ -f "${dir}/${name}" ]; then
      rm -f "${dir}/${name}"
      echo "==> Removed binary: ${dir}/${name}"
    fi
    rm -f "${dir}/${name}".old.* 2>/dev/null || true
  done <<EOF
$search_dirs
EOF

  clibm_unregister_plugin "$name" "$(pwd)" ${extra_dirs[@]+"${extra_dirs[@]}"}
  clibm_path_remove "$name"
  if [ -n "$CLIBM_REMOVED_PATH_DIRS" ]; then
    while IFS= read -r rdir; do
      if [ -z "$rdir" ]; then
        continue
      fi
      echo "==> Removed user PATH entry: $rdir"
      if [ -d "$rdir" ] && [ -z "$(ls -A "$rdir" 2>/dev/null || true)" ]; then
        rmdir "$rdir" 2>/dev/null || true
      fi
    done <<EOF
$CLIBM_REMOVED_PATH_DIRS
EOF
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
echo " CLIBM v0.1.0 -- Interactive Core Installation Wizard (UTF-8)"
echo "===================================================================="

# 1. Core Mode (Pure vs With Default Plugins)
if [ -z "$PROFILE" ]; then
  if [ "$NON_INTERACTIVE" = true ]; then
    PROFILE="pure"
  else
    echo ""
    echo "[1/5] Select CLIBM Core Installation Profile:"
    echo "  1) Pure Core (without plugins -- 100% pure engine) [Default]"
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
    echo "  2) Portugues (pt-BR)"
    echo "  3) Espanol (es)"
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
    echo "  1) Yes -- Record every change individually as a BLAKE3 commit [Default]"
    echo "  2) No  -- Require manual 'clibm commit -m ...'"
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

clibm_build_and_stage "clibm" "" "${INSTALL_DIR}/clibm" "$SCRIPT_DIR" "$RAW_BASE_URL"
BUILT_COMMIT="$CLIBM_LAST_BUILD_COMMIT"
if [ -n "${CLIBM_FORCE_REMOTE:-}" ] || ! clibm_resolve_local_source_dir "clibm" "" "$SCRIPT_DIR" >/dev/null; then
  SOURCE_REPO=$(clibm_git_source_url)
  SOURCE_REF=$(clibm_git_source_ref)
else
  SOURCE_REPO="$SCRIPT_DIR"
  SOURCE_REF=""
fi
CORE_VERSION=$("${INSTALL_DIR}/clibm" --version 2>/dev/null | awk '{ print $NF }' || true)

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
echo " Installation Summary (Encoding: UTF-8):"
echo "   Profile        : $PROFILE"
echo "   Language       : $LANGUAGE"
echo "   Auto-Commit    : $AUTO_COMMIT"
echo "   Install Path   : $INSTALL_DIR"
echo "   User PATH      : $PATH_STATUS"
echo "--------------------------------------------------------------------"

# Persist global installation defaults in ~/.clibm/install-config.json (UTF-8)
GLOBAL_CFG_DIR="${CLIBM_HOME:-${HOME}}/.clibm"
mkdir -p "${GLOBAL_CFG_DIR}/plugins"

json_or_null() {
  if [ -n "$1" ]; then printf '"%s"' "$(clibm_json_escape "$1")"; else printf 'null'; fi
}
{
  printf '{\n'
  printf '  "install_profile": "%s",\n' "$PROFILE"
  printf '  "language": "%s",\n' "$LANGUAGE"
  printf '  "auto_commit": %s,\n' "$AUTO_COMMIT"
  printf '  "install_path": %s,\n' "$(json_or_null "$INSTALL_DIR")"
  printf '  "path_entry_added": %s,\n' "$PATH_ENTRY_ADDED"
  printf '  "mcp_transport": "stdio",\n'
  printf '  "source_repo": %s,\n' "$(json_or_null "$SOURCE_REPO")"
  printf '  "source_ref": %s,\n' "$(json_or_null "$SOURCE_REF")"
  printf '  "source_commit": %s,\n' "$(json_or_null "$BUILT_COMMIT")"
  printf '  "version": %s\n' "$(json_or_null "$CORE_VERSION")"
  printf '}\n'
} > "${GLOBAL_CFG_DIR}/install-config.json"

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
  # Pure core: initialize empty plugin registry if none exists (UTF-8)
  if [ ! -f "${GLOBAL_CFG_DIR}/plugins/registry.json" ]; then
    printf '[]\n' > "${GLOBAL_CFG_DIR}/plugins/registry.json"
  fi
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
