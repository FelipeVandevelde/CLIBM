# clibm-plugin-template — starter for your own plugin

A minimal, dependency-light CLIBM plugin that shows the whole plugin contract. Copy it, or
generate a new plugin with `clibm plugin new`.

## How CLIBM talks to a plugin

1. **Registration.** The installer runs `<plugin> --manifest --executable <path> [--hooks a,b]`
   and stores the printed JSON line in `~/.clibm/plugins/registry.json`:

   ```json
   {"name":"clibm-plugin-template","version":"0.1.0","description":"...","enabled":true,"hooks":["manual-run"],"executable":"/home/me/.local/bin/clibm-plugin-template","tools":[]}
   ```

   `tools` may declare MCP tools (see the `clibm-mcp` README).

2. **Calls.** `clibm plugin run <plugin> [-- args]` and lifecycle hooks start the binary with
   `CLIBM_PROJECT_ROOT` set and send one request on stdin:

   ```json
   {"contract":"CLIBM_PLUGIN_CONTRACT_V1","event":"pre-move","tree_hash":"…","args":[],
    "context":{"task_id":"APP-101","board_id":"BRD-01","from":"todo","to":"in-progress","force":false}}
   ```

   The plugin answers on stdout:

   ```json
   {"ok": true, "output": "..."}
   ```

3. **Hooks.** Events: `pre-commit`, `post-commit`, `pre-move`, `on-task-move`,
   `on-task-depend`, `on-workspace-switch`, `on-export`. Answering `ok: false` to a `pre-*`
   event **blocks** the action (the user sees your `output`); other events are informational.
   Each call has a time limit (`hook_timeout_secs`, default 10). Users can skip hooks with
   `clibm --no-hooks` or disable a plugin per project with `clibm plugin disable <name>`.

Rust plugins that use the core library can rely on `clibm_core::plugin_sdk::PluginRun`,
which reads the request and writes the reply for you.

## Demo switches

To see a veto in action without writing code:

```bash
cd plugins/plugin-template
./install.sh -y --hooks pre-move,post-commit   # PowerShell: .\install.ps1 -Yes -Hooks pre-move,post-commit
cd ~/my-project
export CLIBM_TEMPLATE_DENY=pre-move            # PowerShell: $env:CLIBM_TEMPLATE_DENY = "pre-move"
clibm board move 101 in-progress               # -> Blocked by plugin 'clibm-plugin-template'
export CLIBM_TEMPLATE_LOG=/tmp/events.log      # appends every received event to that file
```

## Quick Install

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/plugin-template/install.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/plugin-template/install.ps1)
```

## Quick Uninstall

### Linux, macOS, WSL2
```bash
curl -fsSL https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/plugin-template/uninstall.sh | bash
```

### Windows (native, PowerShell)
```powershell
iex (irm https://raw.githubusercontent.com/FelipeVandevelde/CLIBM/refs/heads/main/rust-clibm/plugins/plugin-template/uninstall.ps1)
```

Or `clibm plugin uninstall template`.

## Creating your own plugin

```bash
cd ~/my-project
clibm plugin new sprint-velocity --description "Tracks velocity" --hooks post-commit,on-task-move
cd plugins/clibm-sprint-velocity
./install.sh            # or .\install.ps1
clibm plugin run sprint-velocity
```

The generated crate already supports `--manifest` and the stdin contract.
