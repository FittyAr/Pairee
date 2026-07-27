# disk-usage.pairee

A command plugin for Pairee that analyses the disk usage of the active
panel's working directory and surfaces the largest offenders.

## Keybinding

| Key | Action |
|-----|--------|
| `Ctrl+D` | Run the disk-usage analysis on the active panel's cwd |

The result is rendered in the preview pane as a sorted table (largest
first), and a toast notification reports how many entries were scanned.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `depth` | `2` | How deep to recurse when measuring each top-level entry (`1` = no recursion). |
| `top_n` | `20` | How many of the largest entries to display. |
| `include_hidden` | `false` | Include hidden files / dot-directories in the calculation. |
| `extra_args` | `""` | Extra arguments appended to the `du` (or PowerShell) invocation. |

## How it works

The plugin spawns an external process to do the heavy lifting:

- On **Linux / macOS** it runs `du -k --max-depth=<depth> <cwd>`.
- On **Windows** it runs a small PowerShell pipeline that walks the
  directory with `Get-ChildItem -Recurse` and reports byte totals.

The output is parsed, sorted by size, trimmed to the top N entries, and
formatted as a `pairee.ui.Table` widget that is pushed into the preview
pane.

## Why trusted?

The plugin needs to spawn `du` / `powershell`, so it runs in **trusted**
mode. You will be asked to trust the plugin on first install.
