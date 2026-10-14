# archive-inspect

A previewer plugin for Pairee that lists the contents of ZIP, TAR.GZ and 7Z
archives directly in the preview pane — without extracting them.

## When does it activate?

The plugin is automatically registered as a previewer for files with the
following extensions:

- `.zip`
- `.tar`, `.tar.gz`, `.tgz`
- `.7z`

When you hover over an archive, the preview pane will show a table of the
archive's contents (path, size, modification date).

## Commands

| Command | Default keys | Action |
|---------|--------------|--------|
| Archive summary | `Ctrl+Alt+e a` (Neovim `<leader>pa`, yazi `Alt+p a`) | Show a quick summary notification for the hovered archive |

The command is also in the command palette. Rebind it in `keybindings.toml`
with its id, `plugin.archive-inspect.summary`.

PgUp / PgDn scroll the listing in the preview pane.

## Settings

Open the **Plugin Manager**, select the plugin and press `c` (or Enter):

| Setting | Default | Description |
|---------|---------|-------------|
| `max_entries` | `500` | Maximum number of entries to display. Larger lists are truncated. |
| `show_hidden` | `false` | Include dotfiles / hidden entries. |
| `sort_by` | `path` | Default sort column: `path`, `size` or `date`. |
| `extra_args` | `""` | Extra arguments appended to the listing tool. |

## Required tools

This plugin calls out to the following binaries (they must be available on
your `PATH`):

- `unzip` for `.zip` files
- `tar` for `.tar`/`.tar.gz`/`.tgz` files
- `7z` for `.7z` files (the 7-Zip CLI, e.g. `p7zip` on Linux)

Because of this, the plugin runs in **trusted** mode — you will be prompted
to trust it on first install.

In **Secure Mode** only the programs declared in the manifest may run, so
`manifest.toml` declares them:

```toml
[permissions]
commands = ["unzip", "tar", "7z"]
```

## How it works

The plugin implements the previewer contract:

- `peek(job)` — detects the archive type, spawns the right listing tool,
  parses the output, sorts/truncates per the user settings, and returns a
  `pairee.ui.Table` widget.
- `entry(args)` — the `summary` command; reads the hovered file from
  `pairee.cx` (the `--- @sync entry` annotation refreshes it) and shows the
  entry count and total uncompressed size.

The listing output is parsed by a small per-format parser, so no external
Lua dependencies are required.

## Examples

### `peek()` on a `.zip` file

Hover over `release-0.7.0.zip` in the active panel. The preview pane
will be replaced with a `pairee.ui.Table` widget like:

```text
┌──────────────────────────────┬───────┬─────────────────────┐
│ Path                         │ Size  │ Modified            │
├──────────────────────────────┼───────┼─────────────────────┤
│ release-0.7.0/CHANGELOG.md   │ 1.2 K │ 2026-07-26 21:14:00 │
│ release-0.7.0/pairee         │ 24 M  │ 2026-07-26 21:14:00 │
│ release-0.7.0/pairee.sig     │  512  │ 2026-07-26 21:14:00 │
└──────────────────────────────┴───────┴─────────────────────┘
```

Scroll larger lists with PgUp / PgDn in the preview pane.

### The `summary` command on a `.zip` file

Press `Ctrl+Alt+e a` while hovering over `release-0.7.0.zip` to get a quick
summary toast like:

```text
release-0.7.0.zip

3 entries
Total size: 24.0 M
```

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| The preview pane shows "Could not list archive contents" | The required tool (`unzip`, `tar`, or `7z`) is missing on `PATH` | Install it: `apt install unzip / p7zip`, `brew install p7zip`, etc. |
| The `peek()` returns an empty table | The archive is password-protected | The plugin does not support encrypted archives; decrypt first or open in a tool that handles the password. |
| The summary shows "Not a supported archive: …" | The hovered file is not a `.zip` / `.tar(.gz)` / `.tgz` / `.7z` | This is expected; the plugin only inspects those four families. |
| Listings look truncated past a few hundred entries | The `max_entries` setting is too low | Raise it in the Plugin Manager settings of `archive-inspect` (default: `500`). |
| Sort order seems wrong | The `sort_by` setting was changed from the default | Restore it to `path`, `size`, or `date` from the same dialog. |
| Plugin fails to load | It needs Pairee 0.11.0 or newer (Lua API 1.3.0) | Update Pairee, then reinstall with `pairee plugin install archive-inspect`. |
