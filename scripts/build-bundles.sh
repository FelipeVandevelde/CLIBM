#!/usr/bin/env bash
# ==============================================================================
# Builds the CLIBM source ZIP bundles listed in rust-clibm/bundles.json.
#
#   scripts/build-bundles.sh [--out <dir>] [--ref <tree-ish>] [--only <name>[,<name>]]
#
# Files come from git, not from the working tree: by default the current index
# (so staged changes are included), or the tree-ish given with --ref. Blobs are
# taken as stored, so text files keep LF endings and never get a BOM, whatever
# core.autocrlf says on this machine.
#
# Output (default <repo>/dist-bundles):
#   release/<name>-<version>.zip + release/SHA256SUMS   (versioned GitHub release)
#   latest/<name>.zip            + latest/SHA256SUMS    (rolling bundles-latest release)
#   bundles.txt                                          (one bundle name per line)
#
# Needs: git, cargo, jq, sha256sum and python3 (or python).
# ==============================================================================
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="${REPO_ROOT}/dist-bundles"
REF=""
ONLY=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT_DIR="$2"; shift 2 ;;
    --ref) REF="$2"; shift 2 ;;
    --only) ONLY="$2"; shift 2 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

for tool in git cargo jq sha256sum; do
  command -v "$tool" >/dev/null 2>&1 || { echo "ERROR: '$tool' is required" >&2; exit 1; }
done
PYTHON="${PYTHON:-}"
if [ -z "$PYTHON" ]; then
  if python3 -c 'import zipfile' >/dev/null 2>&1; then PYTHON=python3
  elif python -c 'import zipfile' >/dev/null 2>&1; then PYTHON=python
  else echo "ERROR: python3 is required" >&2; exit 1
  fi
fi

# jq without the carriage returns a native Windows jq.exe adds to its output.
jqr() { jq "$@" | tr -d '\r'; }

# Native path for Windows Python under Git Bash; unchanged elsewhere.
native_path() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s\n' "$1"; fi
}

MANIFEST_REL="rust-clibm/bundles.json"
cd "$REPO_ROOT"

if [ -n "$REF" ]; then
  TREE="$(git rev-parse "${REF}^{tree}")"
else
  TREE="$(git write-tree)"
fi

# Fixed timestamp for every ZIP entry: the HEAD commit time, so the same tree
# gives the same ZIP bytes.
SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct HEAD)}"
export SOURCE_DATE_EPOCH

rm -rf "$OUT_DIR"
EXPORT_DIR="${OUT_DIR}/export"
STAGE_DIR="${OUT_DIR}/stage"
mkdir -p "$EXPORT_DIR" "$STAGE_DIR" "${OUT_DIR}/release" "${OUT_DIR}/latest"

echo "==> Exporting rust-clibm/ and LICENSE.txt from tree ${TREE:0:12}"
git -c core.autocrlf=false archive --format=tar "$TREE" rust-clibm LICENSE.txt | tar -x -C "$EXPORT_DIR"
SRC="${EXPORT_DIR}/rust-clibm"
[ -f "${SRC}/bundles.json" ] || { echo "ERROR: ${MANIFEST_REL} is not in the tree (git add it first)" >&2; exit 1; }
MANIFEST="${SRC}/bundles.json"

VERSION="$(awk '
  /^\[package\]/ { in_pkg = 1; next }
  /^\[/          { in_pkg = 0 }
  in_pkg && /^version[[:space:]]*=/ { gsub(/.*=[[:space:]]*"|".*/, ""); print; exit }
' "${SRC}/Cargo.toml")"
[ -n "$VERSION" ] || { echo "ERROR: could not read the version from rust-clibm/Cargo.toml" >&2; exit 1; }
echo "==> Version ${VERSION}"

# Make sure every locked crate is in the local cache, so trimming each bundle's
# Cargo.lock can run offline and never picks newer versions.
echo "==> cargo fetch --locked"
(cd "$SRC" && cargo fetch --locked --quiet)

# Core files shared by every workspace bundle: everything tracked in rust-clibm/
# except plugins/ (catalog.json is added back: install.rs includes it at compile
# time) and tests/.
copy_core() {
  local dest="$1"
  mkdir -p "$dest"
  (cd "$SRC" && find . -mindepth 1 -maxdepth 1 ! -name plugins ! -name tests ! -name bundles.json -exec cp -R {} "$dest/" \;)
  mkdir -p "${dest}/plugins"
  cp "${SRC}/plugins/catalog.json" "${dest}/plugins/catalog.json"
  cp "${EXPORT_DIR}/LICENSE.txt" "${dest}/LICENSE.txt"
}

# Rewrites the [workspace] members list of <Cargo.toml> to the given members.
set_workspace_members() {
  local toml="$1"; shift
  "$PYTHON" - "$(native_path "$toml")" "$@" <<'PY'
import re, sys
path, members = sys.argv[1], sys.argv[2:]
with open(path, encoding="utf-8", newline="") as f:
    text = f.read()
body = "".join(f'    "{m}",\n' for m in members)
new, n = re.subn(r"(?ms)^members\s*=\s*\[.*?\]", "members = [\n" + body + "]", text, count=1)
if n != 1:
    sys.exit(f"no [workspace] members list in {path}")
with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(new)
PY
}

# Drops the packages a bundle no longer uses from its Cargo.lock without
# upgrading anything else, so `cargo build --locked` keeps working.
trim_lock() {
  local dir="$1"
  (cd "$dir" && cargo update --workspace --offline --quiet)
}

# Writes <zip> from <stage_parent>/<top>/ with sorted entries, directory entries,
# a fixed timestamp and Unix modes (0755 for *.sh, 0644 otherwise).
make_zip() {
  local zip="$1" stage_parent="$2" top="$3"
  "$PYTHON" - "$(native_path "$zip")" "$(native_path "$stage_parent")" "$top" <<'PY'
import os, sys, time, zipfile
zip_path, parent, top = sys.argv[1:4]
stamp = time.gmtime(max(int(os.environ.get("SOURCE_DATE_EPOCH", "315532800")), 315532800))[:6]
root = os.path.join(parent, top)
if not os.path.isdir(root):
    sys.exit(f"bundle folder not found: {root}")
entries = []
for dirpath, dirnames, filenames in os.walk(root):
    dirnames.sort()
    rel_dir = os.path.relpath(dirpath, parent).replace(os.sep, "/")
    entries.append((rel_dir + "/", None))
    for name in sorted(filenames):
        entries.append((f"{rel_dir}/{name}", os.path.join(dirpath, name)))
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
    for arcname, src in sorted(entries):
        info = zipfile.ZipInfo(arcname, date_time=stamp)
        info.create_system = 3
        if src is None:
            info.external_attr = (0o40755 << 16) | 0x10
            zf.writestr(info, b"")
        else:
            mode = 0o755 if arcname.endswith(".sh") else 0o644
            info.external_attr = (0o100000 | mode) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            with open(src, "rb") as f:
                zf.writestr(info, f.read())
PY
}

# Fails when a staged text file has a BOM or a carriage return.
check_text_files() {
  local dir="$1" bad=0 f
  while IFS= read -r -d '' f; do
    if [ "$(head -c 3 "$f" | od -An -tx1 | tr -d ' \n')" = "efbbbf" ]; then
      echo "ERROR: UTF-8 BOM in ${f#"$STAGE_DIR"/}" >&2; bad=1
    fi
    if grep -q $'\r' "$f"; then
      echo "ERROR: CRLF line endings in ${f#"$STAGE_DIR"/}" >&2; bad=1
    fi
  done < <(find "$dir" -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.sh' -o -name '*.ps1' -o -name '*.md' -o -name '*.json' -o -name '*.html' -o -name '*.lock' -o -name '*.txt' \) -print0)
  return $bad
}

names=()
while IFS= read -r n; do names+=("$n"); done < <(jqr -r '.bundles[].name' "$MANIFEST")
if [ -n "$ONLY" ]; then
  IFS=',' read -r -a wanted <<<"$ONLY"
  for w in "${wanted[@]}"; do
    printf '%s\n' "${names[@]}" | grep -qx "$w" || { echo "ERROR: unknown bundle '$w'" >&2; exit 2; }
  done
  names=("${wanted[@]}")
fi

for name in "${names[@]}"; do
  bundle="$(jqr -c --arg n "$name" '.bundles[] | select(.name == $n)' "$MANIFEST")"
  stage="${STAGE_DIR}/${name}"
  top="${stage}/${name}"
  echo "==> ${name}"

  standalone="$(jqr -r '.standalone // empty' <<<"$bundle")"
  members_kind="$(jqr -r '.members | type' <<<"$bundle")"

  if [ -n "$standalone" ]; then
    mkdir -p "$top"
    cp -R "${SRC}/${standalone}/." "$top/"
    cp "${EXPORT_DIR}/LICENSE.txt" "${top}/LICENSE.txt"
    cp "${SRC}/Cargo.lock" "${top}/Cargo.lock"
    # Its own workspace root, so an unpacked copy never joins a parent workspace.
    if ! grep -q '^\[workspace\]' "${top}/Cargo.toml"; then
      printf '\n[workspace]\n' >> "${top}/Cargo.toml"
    fi
    trim_lock "$top"
  elif [ "$members_kind" = "string" ]; then
    # "all": the whole workspace as it is in the repository.
    mkdir -p "$top"
    (cd "$SRC" && find . -mindepth 1 -maxdepth 1 ! -name bundles.json -exec cp -R {} "$top/" \;)
    cp "${EXPORT_DIR}/LICENSE.txt" "${top}/LICENSE.txt"
  else
    copy_core "$top"
    members=()
    while IFS= read -r m; do members+=("$m"); done < <(jqr -r '.members[]' <<<"$bundle")
    for m in "${members[@]}"; do
      [ "$m" = "." ] && continue
      mkdir -p "${top}/$(dirname "$m")"
      cp -R "${SRC}/${m}" "${top}/${m}"
    done
    if [ "$(jqr -r '.tests // false' <<<"$bundle")" = "true" ]; then
      cp -R "${SRC}/tests" "${top}/tests"
    fi
    set_workspace_members "${top}/Cargo.toml" "${members[@]}"
    trim_lock "$top"
  fi

  check_text_files "$top"
  make_zip "${OUT_DIR}/release/${name}-${VERSION}.zip" "$stage" "$name"
  cp "${OUT_DIR}/release/${name}-${VERSION}.zip" "${OUT_DIR}/latest/${name}.zip"
  printf '%s\n' "$name" >> "${OUT_DIR}/bundles.txt"
done

(cd "${OUT_DIR}/release" && sha256sum -- *.zip > SHA256SUMS)
(cd "${OUT_DIR}/latest" && sha256sum -- *.zip > SHA256SUMS)
printf '%s\n' "$VERSION" > "${OUT_DIR}/VERSION"
rm -rf "$EXPORT_DIR"

echo
echo "Built ${#names[@]} bundle(s) for version ${VERSION} in ${OUT_DIR}:"
(cd "${OUT_DIR}/release" && ls -l -- *.zip | awk '{ printf "  %-40s %8d bytes\n", $NF, $5 }')
