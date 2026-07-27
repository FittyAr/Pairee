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
