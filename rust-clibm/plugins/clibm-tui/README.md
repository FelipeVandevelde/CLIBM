# clibm-tui — terminal Kanban board

An interactive Kanban board for the terminal (Ratatui + Crossterm). It reads and writes the
project through the CLIBM core library, so every move follows the same rules as the CLI:
dependencies, WIP limits, plugin hooks and automatic commits.

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/install.ps1)
```

From a clone: `cd plugins/clibm-tui && ./install.sh` (or `.\install.ps1`), or
`clibm plugin install tui`. The plugin is built from the whole Cargo workspace because it
uses the core library.

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/clibm-tui/uninstall.ps1)
```

Or `clibm plugin uninstall tui`.

## Usage

```bash
cd ~/my-project
clibm plugin run tui      # or: clibm-tui
```

It shows the **active project's board** with its own columns and WIP limits, and picks up
changes made elsewhere (CLI, MCP, web) every two seconds. The header shows the active
workspace and project (`workspace › CODE - project`).

| Key | Action |
|---|---|
| `h` / `Left`, `l` / `Right` | Previous / next column |
| `k` / `Up`, `j` / `Down` | Previous / next task |
| `m` | Move the selected task one column to the right |
| `b` | Move the selected task one column to the left |
| `c` | Commit the working tree (does nothing when there is nothing to commit) |
| `f` | Verify every stored object (`fsck`) |
| `r` | Reload now |
| `w` | Open the workspace & project picker |
| `[` / `]` | Previous / next project of the active workspace |
| `W` | Next workspace (alphabetically) |
| `q` / `Esc` | Quit |

A refused move (blocking dependency, full column, plugin veto) is explained in the status
line; use the CLI with `--force` to override.

### Workspaces and projects

`w` opens a picker with the workspaces on the left (`*` marks the active one) and the
projects of the highlighted workspace on the right.

| Key | Action |
|---|---|
| `Tab`, `h` / `l` | Change pane |
| `j` / `k` | Select |
| `Enter` | Switch to the selected workspace or project |
| `n` | New workspace (left pane) / new project in the highlighted workspace (right pane) |
| `e` | Edit the name and description (a project's code cannot change) |
| `d` | Delete the selected item |
| `Esc` / `q` | Back to the board |

In a form, `Tab` moves between fields, `Space` ticks a box, `Ctrl+U` clears a field, `Enter`
saves and `Esc` cancels. When the core refuses a change (a name or code already in use, an
invalid code) the form stays open and shows why.

Deleting asks for confirmation (`y`). A project with tasks needs a project to receive them,
and a workspace with tasks needs a workspace to receive its projects (pick it with `j`/`k`).
The last project of a workspace and the last workspace cannot be deleted. Everything stays
in the history, so `clibm restore` can bring it back.
