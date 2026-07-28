# archive-inspect.pairee

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

## Keybindings

| Key | Action |
|-----|--------|
| `F2` | Show a quick summary notification for the hovered archive |

## Settings

You can tweak the plugin from the Pairee **Options** dialog (Plugins tab):

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

## How it works

The plugin implements the previewer contract:

- `peek(job)` — detects the archive type, spawns the right listing tool,
  parses the output, sorts/truncates per the user settings, and returns a
  `pairee.ui.Table` widget.
- `seek(job)` — re-emits the cached entries when the user scrolls.
- `entry()` — invoked by `F2`; shows a popup with the entry count and total
  uncompressed size.

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

Use `seek(job)` (or scroll with the keyboard in the preview pane)
to paginate through larger lists.

### `entry()` on a `.zip` file

Press `F2` while hovering over `release-0.7.0.zip` to get a quick
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
| `F2` shows "Not a supported archive: …" | The hovered file is not a `.zip` / `.tar(.gz)` / `.tgz` / `.7z` | This is expected; the plugin only inspects those four families. |
| Listings look truncated past a few hundred entries | The `max_entries` setting is too low | Bump it from `Options → Plugins → archive-inspect` (default: `500`). |
| Sort order seems wrong | The `sort_by` setting was changed from the default | Restore it to `path`, `size`, or `date` from the same dialog. |
| Plugin fails to load | The `manifest.toml` [files] hash for `main.lua` no longer matches the on-disk file | The lockfile was tampered with; reinstall via `pairee plugin install archive-inspect.pairee`. |
