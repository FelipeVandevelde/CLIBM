# CLIBM — Rust workspace

The native implementation of CLIBM: the `clibm` command line, the `clibm_core` library and the
official plugins. The full user guide (commands, plugins, hooks, storage layout) is in the
[project README](../README.md).

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/uninstall.ps1)
```

## Installing from this folder

```bash
./install.sh                         # interactive
./install.sh -y --no-modify-path     # non-interactive, PATH untouched
./install.sh --with-defaults -y      # + dep-audit, export-md, workflow-lint
```

```powershell
.\install.ps1
.\install.ps1 -Yes -NoModifyPath
.\install.ps1 -Profile with-defaults -Yes
```

Requires the Rust toolchain (`cargo`, from https://rustup.rs). In a clone the installers build
the local sources (`cargo build --release -p <crate>`); otherwise they run
`cargo install --git` against `CLIBM_GIT_URL` / `CLIBM_GIT_REF` (default: the official
repository, branch `main`). Choices and the build's source (repository, branch, commit,
version) are recorded in `~/.clibm/install-config.json`.

## Updating

```bash
clibm update --check        # is there a newer commit?
clibm update [--plugins]    # rebuild and replace the installed binaries
./update.sh                 # in a clone: git pull, then update from the clone
.\update.ps1                # same on Windows
```

## Workspace layout

| Path | Contents |
|---|---|
| `src/` | `clibm` binary (`main.rs`, `cli.rs`) and the `clibm_core` library |
| `src/canonical.rs`, `cas.rs` | canonical JSON, BLAKE3 object store, refs, fsck |
| `src/engine.rs`, `versioning.rs`, `index.rs` | tasks, rules, status, commits, 3-way merge, index |
| `src/plugin.rs`, `plugin_sdk.rs` | plugin registry, install/uninstall, lifecycle hooks, plugin helper |
| `src/install.rs`, `update.rs` | install record and `clibm update` |
| `tests/` | end-to-end tests of the `clibm` binary (isolated temp folders and home) |
| `plugins/` | official plugins (each with its own README and install scripts) |
| `scripts/env-path.ps1`, `env-path.sh` | shared helpers embedded in every install/uninstall script |
| `scripts/sync-env-path.ps1` | copies the helpers into the scripts (`-Check` reports drift) |

## Development

```bash
cargo build --workspace
cargo test --workspace
powershell -File scripts/sync-env-path.ps1 -Check
```

Tests never touch your real `~/.clibm`: they point `CLIBM_HOME` at a temporary folder. Set
`CLIBM_HOME` yourself to try installers or plugins in a sandbox, together with
`-NoModifyPath` / `--no-modify-path`.

## License

MIT — see [LICENSE.txt](../LICENSE.txt).
