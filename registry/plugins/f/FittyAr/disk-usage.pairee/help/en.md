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

## Examples

### Default run (depth=2, top_n=20)

With the cursor parked on a typical project folder, press `Ctrl+D`.
The preview pane will be replaced with a `pairee.ui.Table` widget
similar to:

```text
┌──────────────────┬──────────────────────────┐
│ Size             │ Path                     │
├──────────────────┼──────────────────────────┤
│ 1.4 G (48%)      │ node_modules/            │
│ 820 M (28%)      │ target/                  │
│ 240 M (8%)       │ .git/                    │
│ 180 M (6%)       │ vendor/                  │
│ …                │ …                        │
└──────────────────┴──────────────────────────┘

/home/me/projects/pairee
Scanned: 18 entries · Total: 2.9 G · Depth: 2

Largest 20 entries:
```

A toast notification at the bottom of the screen reports the
total scanned count and the top-N limit.

### Tweak the recursion depth

Set `depth = 1` in the dialog (`Options → Plugins → disk-usage`)
to only measure the **top level** of the cwd (one level of
recursion, no nested file counts). Useful for "which subdirectory
should I delete to free the most space?".

### Include hidden files

Set `include_hidden = true` if your home directory has a
`~/.cache` or `~/.local/share` that you want to surface in the
report.

### Pass extra arguments to `du`

If your version of `du` supports `--exclude`, you can pass it
through `extra_args` so that artifact directories like
`build-artifacts/` are skipped:

```text
extra_args = "--exclude=build-artifacts"
```

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Toast says "Required tool 'du' is not on PATH" | The binary is missing or the `$PATH` doesn't include `/usr/bin` | Install `coreutils` (Linux) or `du` is preinstalled on macOS. On Windows, ensure `du` is available (Cygwin / MSYS) — otherwise the PowerShell branch should kick in. |
| Report is empty (`No scannable entries were found in this directory`) | The cwd is empty or you set `depth = 0` | Bump `depth` to at least `1`, or change to a folder with files. |
| Permission-denied errors are silently dropped | The plugin ignores files it can't read (avoids spamming the report with system noise) | Re-run as administrator if you need those entries, or exclude the offending subtree with `extra_args`. |
| Plugin fails to load | The `manifest.toml` [files] hash for `main.lua` no longer matches the on-disk file | Reinstall via `pairee plugin install disk-usage.pairee`. |
| Result is `0 B` for every entry | `depth` is too low for your layout, or the cwd is on a different filesystem (e.g. a network share) | Try `depth = 3` or higher; on remote FSes, run from a local copy. |
