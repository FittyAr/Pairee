# recent-files.pairee

A mixed hook + command plugin that silently tracks the files and
directories you visit, and lets you jump back to any of them with a
single keypress.

## How it works

While Pairee is running, the plugin:

1. Subscribes to `on_cd` and (optionally) `on_hover` events.
2. Records each visit with a timestamp.
3. Debounces and writes the list to a JSON file in your Pairee config
   directory (defaults to `recent-files.json`).
4. Publishes a `recent-files:added` pub/sub event so other plugins can
   react to the new visit (e.g. an "open in editor" plugin could keep
   its MRU list in sync without duplicating state).

## Keybinding

| Key | Action |
|-----|--------|
| `Ctrl+R` | Open the recent-files picker in the active panel |

The picker is a `which` prompt: type the number of the entry, or use
arrow keys, then `Enter` to jump. `Esc` cancels.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `max_entries` | `50` | Maximum number of recent entries to keep on disk. |
| `record_dirs` | `true` | Record directory changes (`on_cd`) in addition to file selections. |
| `record_hover` | `false` | Also record every cursor hover. Off by default — can be noisy on big directories. |
| `persist_path` | `""` | Override the JSON history file path. Empty = use the Pairee default. |

## Public API for other plugins

Other plugins can call into this one through the `pairee.sync` boundary:

```lua
local recent = require("recent-files.pairee")
local items  = recent:list(5)   -- 5 most recent entries
```

The plugin also publishes a `recent-files:added` event:

```lua
pairee.ps.sub("recent-files:added", function(entry)
    -- entry.path, entry.kind, entry.at
end)
```

## Why no trust required?

The plugin only uses the Pairee FS API (`pairee.fs.read` / `pairee.fs.write`)
to persist state. It does **not** spawn external processes, so it runs
safely in untrusted (sandboxed) mode.
