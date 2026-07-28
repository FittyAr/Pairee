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

## Examples

### The on-disk state

`recent-files.json` lives in the Pairee config directory and looks
like:

```json
{
  "_recent_files_v1": true,
  "entries": [
    { "path": "/home/me/projects/pairee", "kind": "dir",  "at": 1722115200 },
    { "path": "/home/me/projects/pairee/README.md", "kind": "file", "at": 1722111600 },
    { "path": "/home/me/Downloads", "kind": "dir",  "at": 1722024000 }
  ]
}
```

The newest entry is always first. The shape is versioned with the
`_recent_files_v1` sentinel so a future migration can detect and
upgrade older files.

### The picker

Press `Ctrl+R` with the cursor over any panel. You will get a
`pairee.which` prompt that lists every recent entry:

```text
[1]  /home/me/projects/pairee
[2]  /home/me/projects/pairee/README.md
[3]  /home/me/Downloads
[4]  /tmp/scratch.txt
```

Type the number (or use the arrow keys) and press `Enter` to jump.
`Esc` cancels.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Toast says "No recent files tracked yet — visit a few directories first" | The state file is empty (fresh install) or its `_recent_files_v1` sentinel is missing | Visit a few directories so `on_cd` populates the list. If the file was hand-edited and lost the sentinel, delete it — the plugin will recreate it on the next `on_cd`. |
| The picker opens but jumps to the wrong directory | Two recent entries share a parent and you picked the wrong one | Re-run the picker and read the index more carefully. The plugin picks the entry by *index*, not by name. |
| Nothing is being recorded | `record_dirs = false` in the plugin settings | Flip it back to `true` from `Options → Plugins → recent-files`. The default is `true`, but a previous user may have turned it off. |
| Disk-usage is also slow because the history grew unbounded | `max_entries` is too high or `record_hover = true` on a big directory | Lower `max_entries` (default: 50), or turn `record_hover` off. The plugin does not write on every keystroke — it debounces — but a 100k-entry file is still a lot to parse. |
| Plugin fails to load | The `manifest.toml` [files] hash for `main.lua` no longer matches the on-disk file | Reinstall via `pairee plugin install recent-files.pairee`. |
| Another plugin cannot subscribe to `recent-files:added` | The subscribing plugin was loaded before this one | Plugins must be loaded in dependency order. The `recent-files.pairee` plugin should appear in `Plugins → Installed` before any plugin that subscribes to its pub/sub event. |
