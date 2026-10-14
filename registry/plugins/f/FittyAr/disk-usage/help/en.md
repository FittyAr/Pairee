# disk-usage

A command plugin for Pairee that analyses the disk usage of the active
panel's working directory and surfaces the largest offenders.

## Command

| Command | Default keys | Action |
|---------|--------------|--------|
| Disk usage report | `Ctrl+Alt+e d` (Neovim `<leader>pd`, yazi `Alt+p d`) | Run the disk-usage analysis on the active panel's folder |

The command is also in the command palette. Rebind it in `keybindings.toml`
with its id, `plugin.disk-usage.report`.

The result is shown in the Quick View of the scanned folder as a sorted
table (largest first), and a notification reports how many entries were
scanned. Remote panels (SFTP, FTP, ...) are not scanned.

## Settings

Open the **Plugin Manager**, select the plugin and press `c` (or Enter):

| Setting | Default | Description |
|---------|---------|-------------|
| `depth` | `2` | How deep `du` lists nested folders (`1` = only the top-level entries). Windows always lists the top level. |
| `top_n` | `20` | How many of the largest entries to display. |
| `include_hidden` | `false` | Include hidden files / dot-directories in the report. |
| `extra_args` | `""` | Extra arguments appended to the `du` invocation (Linux / macOS only). |

## How it works

- On **Linux / macOS** it spawns `du -k -d <depth> <cwd>` (`-d` works with both GNU and BSD `du`).
- On **Windows** it walks the directory natively with `pairee.fs.read_dir`
  and sums file sizes per top-level entry (symlinks are not followed). No
  process is spawned.

The output is parsed, sorted by size, trimmed to the top N entries, and
shown with `pairee.preview_widget` as a summary above a `pairee.ui.Table`.

## Why trusted?

The plugin needs to spawn `du` (Linux / macOS), so it runs in **trusted**
mode. You will be asked to trust the plugin on first install.

In **Secure Mode** only declared programs may run; `manifest.toml` declares:

```toml
[permissions]
commands = ["du"]
```

## Examples

### Default run (depth=2, top_n=20)

Inside a typical project folder, press `Ctrl+Alt+e d`. The Quick View
of the folder shows a report similar to:

```text
/home/me/projects/pairee
Scanned: 18 entries · Total: 2.9 GB
Largest 18 entries:

Size              Path
1.4 GB (48%)      /home/me/projects/pairee/node_modules
820.0 MB (28%)    /home/me/projects/pairee/target
240.0 MB (8%)     /home/me/projects/pairee/.git
180.0 MB (6%)     /home/me/projects/pairee/vendor
…                 …
```

A notification reports the total scanned count and how many entries are
shown.

### Tweak the recursion depth

Set `depth = 1` in the plugin settings (Plugin Manager → `disk-usage` → `c`)
to only list the **top level** of the folder (each entry still counts
everything below it). Useful for "which subdirectory
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
| Toast says "Required tool 'du' is not on PATH" | The binary is missing or the `$PATH` doesn't include `/usr/bin` | Install `coreutils` (Linux) or `du` is preinstalled on macOS. Windows does not need `du` (native scan). |
| Report is empty (`No scannable entries were found in this directory`) | The cwd is empty or you set `depth = 0` | Bump `depth` to at least `1`, or change to a folder with files. |
| Permission-denied errors are silently dropped | The plugin ignores files it can't read (avoids spamming the report with system noise) | Re-run as administrator if you need those entries, or exclude the offending subtree with `extra_args`. |
| Plugin fails to load | It needs Pairee 0.11.0 or newer (Lua API 1.3.0) | Update Pairee, then reinstall with `pairee plugin install disk-usage`. |
| Result is `0 B` for every entry | `depth` is too low for your layout, or the cwd is on a different filesystem (e.g. a network share) | Try `depth = 3` or higher; on remote FSes, run from a local copy. |
