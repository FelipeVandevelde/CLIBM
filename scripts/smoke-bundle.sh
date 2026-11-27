#!/usr/bin/env bash
# ==============================================================================
# Smoke-tests one CLIBM source ZIP built by scripts/build-bundles.sh.
#
#   scripts/smoke-bundle.sh <bundles-dir> <bundle-name>
#
# Unpacks <bundles-dir>/latest/<name>.zip into a temporary folder, runs
# `cargo build --release --locked`, installs it with the bundle's own installer
# scripts, runs clibm init / status / a task, checks the plugin, then runs the
# core uninstaller with --purge and checks that nothing is left behind.
#
# HOME and CLIBM_HOME point at the temporary folder for the whole run, so the
# installers and the uninstaller never touch the real user profile. CARGO_HOME
# and RUSTUP_HOME keep their real values while building and point at the
# temporary folder while uninstalling (the uninstaller cleans <CARGO_HOME>/bin).
# ==============================================================================
set -euo pipefail

BUNDLES_DIR="${1:?usage: smoke-bundle.sh <bundles-dir> <bundle-name>}"
NAME="${2:?usage: smoke-bundle.sh <bundles-dir> <bundle-name>}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="${REPO_ROOT}/rust-clibm/bundles.json"
ZIP="$(cd "$BUNDLES_DIR" && pwd)/latest/${NAME}.zip"
[ -f "$ZIP" ] || { echo "ERROR: $ZIP not found" >&2; exit 1; }

jqr() { jq "$@" | tr -d '\r'; }
bundle="$(jqr -c --arg n "$NAME" '.bundles[] | select(.name == $n)' "$MANIFEST")"
[ -n "$bundle" ] || { echo "ERROR: unknown bundle '$NAME'" >&2; exit 1; }
plugin="$(jqr -r '.plugin // empty' <<<"$bundle")"
folder="$(jqr -r '.folder // empty' <<<"$bundle")"
standalone="$(jqr -r '.standalone // empty' <<<"$bundle")"
members_kind="$(jqr -r '.members | type' <<<"$bundle")"
smoke="$(jqr -r '.smoke // "installed"' <<<"$bundle")"

EXE=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) EXE=".exe" ;; esac

REAL_CARGO_HOME="${CARGO_HOME:-${HOME}/.cargo}"
REAL_RUSTUP_HOME="${RUSTUP_HOME:-${HOME}/.rustup}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/clibm-smoke-${NAME}.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
export HOME="${WORK}/home" CLIBM_HOME="${WORK}/home"
export CARGO_HOME="$REAL_CARGO_HOME" RUSTUP_HOME="$REAL_RUSTUP_HOME"
BIN="${WORK}/bin"
PROJECT="${WORK}/project"
mkdir -p "$HOME" "$BIN" "$PROJECT"

step() { echo; echo "==> [${NAME}] $*"; }
fail() { echo "SMOKE FAILED [${NAME}]: $*" >&2; exit 1; }

step "Unpacking $(basename "$ZIP")"
unzip -q "$ZIP" -d "$WORK"
ROOT="${WORK}/${NAME}"
[ -d "$ROOT" ] || fail "the ZIP has no top-level ${NAME}/ folder"
for f in $(cd "$ROOT" && find . -name '*.sh'); do
  [ -x "${ROOT}/${f}" ] || [ -n "$EXE" ] || fail "${f} is not executable in the ZIP"
done

step "cargo build --release --locked"
(cd "$ROOT" && cargo build --release --locked --workspace)

install_plugin_from() {
  local dir="$1"
  case "$plugin" in
    clibm-web) (cd "$dir" && bash ./install.sh -y --install-dir "$BIN" --no-modify-path --port 7878) ;;
    clibm-mcp) (cd "$dir" && bash ./install.sh -y --install-dir "$BIN" --no-modify-path --transport stdio) ;;
    *)         (cd "$dir" && bash ./install.sh -y --install-dir "$BIN" --no-modify-path) ;;
  esac
}

if [ -n "$standalone" ]; then
  step "Installing ${plugin} with its own installer"
  install_plugin_from "$ROOT"
else
  profile="--pure"
  [ "$members_kind" = "string" ] && profile="--with-defaults"
  step "Installing the core (${profile})"
  (cd "$ROOT" && bash ./install.sh "$profile" -y --lang en --auto-commit true --install-dir "$BIN" --no-modify-path)
  [ -x "${BIN}/clibm${EXE}" ] || fail "clibm was not installed in ${BIN}"
  if [ -n "$plugin" ]; then
    step "Installing ${plugin} from plugins/${folder}"
    install_plugin_from "${ROOT}/plugins/${folder}"
  fi
fi

if [ -n "$plugin" ]; then
  [ -x "${BIN}/${plugin}${EXE}" ] || fail "${plugin} was not installed in ${BIN}"
fi

if [ -x "${BIN}/clibm${EXE}" ]; then
  CLIBM="${BIN}/clibm${EXE}"
  step "clibm init / status / task add / board move"
  (
    cd "$PROJECT"
    "$CLIBM" --version
    "$CLIBM" init
    "$CLIBM" task add "Smoke test task"
    "$CLIBM" board move CLIBM-101 in-progress
    "$CLIBM" status
    "$CLIBM" plugin list
  )
  if [ "$members_kind" = "string" ]; then
    for p in dep-audit export-md workflow-lint; do
      (cd "$PROJECT" && "$CLIBM" plugin run "$p")
    done
  fi
fi

case "$smoke" in
  run)
    step "clibm plugin run ${plugin#clibm-}"
    (cd "$PROJECT" && "${BIN}/clibm${EXE}" plugin run "${plugin#clibm-}")
    ;;
  version)
    step "${plugin} --version"
    "${BIN}/${plugin}${EXE}" --version
    ;;
esac

step "Uninstalling (--purge)"
(
  cd "$PROJECT"
  CARGO_HOME="${WORK}/home/.cargo" RUSTUP_HOME="${WORK}/home/.rustup" \
    bash "${ROOT}/uninstall.sh" -y --purge
)

leftover="$(find "$BIN" -maxdepth 1 -name 'clibm*' 2>/dev/null || true)"
[ -z "$leftover" ] || fail "binaries left in ${BIN}: ${leftover}"
if [ -z "$standalone" ]; then
  [ ! -e "${CLIBM_HOME}/.clibm" ] || fail "${CLIBM_HOME}/.clibm is still there"
  [ ! -e "${PROJECT}/.clibm" ] || fail "--purge did not remove ${PROJECT}/.clibm"
fi

echo
echo "SMOKE OK [${NAME}]"
