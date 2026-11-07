#!/usr/bin/env bash
# ==============================================================================
# clibm-mcp — Uninstallation Script (Linux / macOS)
# Encoding: UTF-8 with BOM (EF BB BF) for cross-platform compatibility
# ==============================================================================
set -euo pipefail

PURGE=""
NON_INTERACTIVE=false
for arg in "$@"; do
  case "$arg" in
    --purge|-p) PURGE="true" ;;
    --keep|-k) PURGE="false" ;;
    -y|--yes) NON_INTERACTIVE=true ;;
  esac
done

# >>> clibm-env-path (generated) >>>
clibm_init_env_path_helper() {
  local home_dir="${CLIBM_HOME:-${HOME}}"
  local lib_dir="${home_dir%/}/.clibm/lib"
  local helper_file="${lib_dir}/env-path.sh"
  local embedded_version=3
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
# clibm-env-path-version: 3
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

clibm_update_registry_file() {
  local reg_file="$1"
  local name="$2"
  local new_obj="$3"
  mkdir -p "$(dirname "$reg_file")"

  if [ ! -f "$reg_file" ] || ! grep -Fq "\"name\"" "$reg_file" 2>/dev/null; then
    printf '\xEF\xBB\xBF[\n  %s\n]\n' "$new_obj" > "$reg_file"
    return 0
  fi

  local bom=$'\xef\xbb\xbf'
  local clean_content
  clean_content=$(tr -d '\r' < "$reg_file" | sed "1s/^${bom}//")

  if ! printf '%s\n' "$clean_content" | grep -Fq "\"name\": \"${name}\"" 2>/dev/null && \
     ! printf '%s\n' "$clean_content" | grep -Fq "\"name\":\"${name}\"" 2>/dev/null; then
    local body
    body=$(printf '%s\n' "$clean_content" | sed '$d')
    printf '\xEF\xBB\xBF%s,\n  %s\n]\n' "$body" "$new_obj" > "$reg_file"
    return 0
  fi

  local tmp_file="${reg_file}.tmp.$$"
  awk -v target_name="\"${name}\"" -v replacement="  ${new_obj}" '
    BEGIN {
      in_obj = 0
      obj_buf = ""
      obj_count = 0
      replaced = 0
    }
    /^[[:space:]]*\{/ {
      in_obj = 1
      obj_buf = $0
      next
    }
    in_obj {
      obj_buf = obj_buf "\n" $0
      if ($0 ~ /^[[:space:]]*\}[[:space:]]*,?[[:space:]]*$/) {
        in_obj = 0
        sub(/,[[:space:]]*$/, "", obj_buf)
        if (index(obj_buf, "\"name\": " target_name) > 0 || index(obj_buf, "\"name\":" target_name) > 0) {
          objs[++obj_count] = replacement
          replaced = 1
        } else {
          objs[++obj_count] = obj_buf
        }
        obj_buf = ""
      }
      next
    }
    END {
      if (!replaced) {
        objs[++obj_count] = replacement
      }
      printf "[\n"
      for (i = 1; i <= obj_count; i++) {
        printf "%s%s\n", objs[i], (i < obj_count ? "," : "")
      }
      printf "]\n"
    }
  ' <<EOF > "$tmp_file"
$clean_content
EOF
  printf '\xEF\xBB\xBF' > "$reg_file"
  cat "$tmp_file" >> "$reg_file"
  rm -f "$tmp_file"
}

clibm_register_global_plugin() {
  local name="$1"
  local description="$2"
  local hooks_csv="$3"
  local executable="$4"
  local version="${5:-0.1.0}"
  local scope="${6:-global}"
  local project_dir="${7:-}"
  local install_dir="${8:-}"
  local home_dir="${CLIBM_HOME:-${HOME}}"
  local global_reg="${home_dir%/}/.clibm/plugins/registry.json"

  local hooks_json=""
  local IFS=','
  local h
  for h in $hooks_csv; do
    h="${h#"${h%%[![:space:]]*}"}"
    h="${h%"${h##*[![:space:]]}"}"
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

  local target_regs=""
  if [ "$scope" != "project-only" ]; then
    target_regs="${global_reg}"$'\n'
  fi

  local proj_dirs
  proj_dirs=$(clibm_find_project_plugin_dirs "$project_dir" "$install_dir" || true)
  local pdir
  if [ -n "$proj_dirs" ]; then
    while IFS= read -r pdir; do
      [ -z "$pdir" ] && continue
      target_regs="${target_regs}${pdir}/registry.json"$'\n'
    done <<EOF
$proj_dirs
EOF
  elif [ "$scope" = "project-only" ]; then
    local base_proj="${project_dir:-$(pwd)}"
    target_regs="${target_regs}${base_proj%/}/.clibm/plugins/registry.json"$'\n'
    target_regs="${target_regs}${global_reg}"$'\n'
  fi

  local bin_dir="${install_dir:-$(dirname "$executable")}"
  if [ -n "$bin_dir" ]; then
    target_regs="${target_regs}${bin_dir%/}/registry.json"$'\n'
  fi

  local rfile
  while IFS= read -r rfile; do
    [ -z "$rfile" ] && continue
    clibm_update_registry_file "$rfile" "$name" "$new_obj"
    echo "==> Registered ${name} in plugin registry: ${rfile}"
  done <<EOF
$(printf '%s' "$target_regs" | awk 'NF && !seen[$0]++')
EOF
}

clibm_unregister_plugin() {
  local name="$1"
  local project_dir="${2:-}"
  shift $(( $# >= 2 ? 2 : $# ))
  local home_dir="${CLIBM_HOME:-${HOME}}"
  local candidate_regs="${home_dir%/}/.clibm/plugins/registry.json
${home_dir%/}/.local/bin/registry.json
${home_dir%/}/.clibm-bin/registry.json"

  local proj_dirs
  proj_dirs=$(clibm_find_project_plugin_dirs "$project_dir" "$@" || true)
  local pdir
  if [ -n "$proj_dirs" ]; then
    while IFS= read -r pdir; do
      [ -z "$pdir" ] && continue
      candidate_regs="${candidate_regs}
${pdir}/registry.json"
    done <<EOF
$proj_dirs
EOF
  fi
  for extra_d in "$@"; do
    [ -z "$extra_d" ] && continue
    candidate_regs="${candidate_regs}
${extra_d%/}/registry.json"
  done

  local rfile
  local bom=$'\xef\xbb\xbf'
  while IFS= read -r rfile; do
    [ -z "$rfile" ] && continue
    [ ! -f "$rfile" ] && continue
    if ! grep -Fq "\"${name}\"" "$rfile" 2>/dev/null; then
      continue
    fi
    local clean_content
    clean_content=$(tr -d '\r' < "$rfile" | sed "1s/^${bom}//")
    local tmp_file="${rfile}.tmp.$$"
    awk -v target_name="\"${name}\"" '
      BEGIN {
        in_obj = 0
        obj_buf = ""
        obj_count = 0
      }
      /^[[:space:]]*\{/ {
        in_obj = 1
        obj_buf = $0
        next
      }
      in_obj {
        obj_buf = obj_buf "\n" $0
        if ($0 ~ /^[[:space:]]*\}[[:space:]]*,?[[:space:]]*$/) {
          in_obj = 0
          sub(/,[[:space:]]*$/, "", obj_buf)
          if (index(obj_buf, "\"name\": " target_name) == 0 && index(obj_buf, "\"name\":" target_name) == 0) {
            objs[++obj_count] = obj_buf
          }
          obj_buf = ""
        }
        next
      }
      END {
        if (obj_count == 0) {
          printf "[]\n"
        } else {
          printf "[\n"
          for (i = 1; i <= obj_count; i++) {
            printf "%s%s\n", objs[i], (i < obj_count ? "," : "")
          }
          printf "]\n"
        }
      }
    ' <<EOF > "$tmp_file"
$clean_content
EOF
    printf '\xEF\xBB\xBF' > "$rfile"
    cat "$tmp_file" >> "$rfile"
    rm -f "$tmp_file"
    echo "==> Removed ${name} from plugin registry: ${rfile}"
  done <<EOF
$(printf '%s\n' "$candidate_regs" | awk 'NF && !seen[$0]++')
EOF
}
CLIBM_ENV_PATH_EOF
  fi
  # shellcheck disable=SC1090
  . "$helper_file"
}
clibm_init_env_path_helper
# <<< clibm-env-path (generated) <<<

echo "===================================================================="
echo " CLIBM-MCP v0.1.0 — Plugin Uninstaller (UTF-8-BOM)"
echo "===================================================================="

if [ -z "$PURGE" ]; then
  if [ "$NON_INTERACTIVE" = true ]; then
    PURGE="false"
  else
    echo "Select Uninstallation Mode for clibm-mcp:"
    echo "  1) Remove clibm-mcp binary only, keep ~/.clibm/mcp-config.json [Default]"
    echo "  2) Remove clibm-mcp binary AND purge ~/.clibm/mcp-config.json (--purge)"
    read -r -p "Select option [1-2, default: 1]: " c
    case "${c:-1}" in
      2|purge) PURGE="true" ;;
      *) PURGE="false" ;;
    esac
  fi
fi

OWNER_DIRS=$(clibm_path_owner_dirs "clibm-mcp" || true)
SEARCH_DIRS=$(
  {
    [ -n "$OWNER_DIRS" ] && printf '%s\n' "$OWNER_DIRS"
    printf '%s\n' "${HOME}/.local/bin" "${HOME}/.cargo/bin" "${HOME}/.clibm-bin"
  } | awk 'NF && !seen[$0]++'
)
while IFS= read -r dir; do
  [ -z "$dir" ] && continue
  if [ -f "${dir}/clibm-mcp" ]; then
    rm -f "${dir}/clibm-mcp"
    echo "==> Removed binary: ${dir}/clibm-mcp"
  fi
done <<EOF
$SEARCH_DIRS
EOF

clibm_unregister_plugin "clibm-mcp" "$(pwd)"
clibm_path_remove "clibm-mcp"
if [ -n "$CLIBM_REMOVED_PATH_DIRS" ]; then
  while IFS= read -r rdir; do
    [ -z "$rdir" ] && continue
    echo "==> Removed user PATH entry: $rdir"
    if [ -d "$rdir" ] && [ -z "$(ls -A "$rdir" 2>/dev/null || true)" ]; then
      rmdir "$rdir" 2>/dev/null || true
    fi
  done <<EOF
$CLIBM_REMOVED_PATH_DIRS
EOF
fi

if [ "$PURGE" = "true" ] && [ -f "${HOME}/.clibm/mcp-config.json" ]; then
  rm -f "${HOME}/.clibm/mcp-config.json"
  echo "==> Purged ~/.clibm/mcp-config.json"
fi
echo "==> clibm-mcp uninstallation complete."
