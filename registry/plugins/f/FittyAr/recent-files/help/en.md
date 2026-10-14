# recent-files

A mixed hook + command plugin that silently tracks the folders (and,
optionally, the files) you visit, and lets you jump back to any of them
with a single keypress.

## How it works

While Pairee is running, the plugin:

1. Subscribes to `on_cd` and (optionally) `on_hover` events.
2. Moves each visited path to the top of its history, keeping at most
   `max_entries` paths.
3. Saves the history with `pairee.state`, in the plugin's private data
   folder, so it survives restarts.
4. Publishes a `recent-files:added` pub/sub event (`{ path, kind }`) so
   other plugins can react to the new visit.

## Command

| Command | Default keys | Action |
|---------|--------------|--------|
| Recent files | `Ctrl+Alt+e r` (Neovim `<leader>pr`, yazi `Alt+p r`) | Open the recent-files picker |

The command is also in the command palette. Rebind it in `keybindings.toml`
with its id, `plugin.recent-files.pick`.

The picker lists the newest 35 entries, each with a key (`1`-`9`, then
`a`-`z`). Press the key to jump there; `Esc` cancels. A folder opens in the
active panel; for a file, the panel opens the folder that holds it.

## Settings

Open the **Plugin Manager**, select the plugin and press `c` (or Enter):

| Setting | Default | Description |
|---------|---------|-------------|
| `max_entries` | `50` | Maximum number of recent entries to keep. |
| `record_dirs` | `true` | Record folder changes (`on_cd`). |
| `record_hover` | `false` | Also record every file under the cursor. Off by default: it can be noisy in big folders. |

## Why no trust required?

The plugin only uses `pairee.state` (its own data folder) and the panel
events. It does **not** spawn processes or read other files, so it runs in
untrusted (sandboxed) mode.

## Listening from another plugin

```lua
pairee.ps.sub("recent-files:added", function(payload, meta)
    -- payload.path, payload.kind ("dir" or "file"); meta.sender == "recent-files"
end)
```

## Examples

### The picker

Press `Ctrl+Alt+e r`. You will get a `pairee.which` prompt that lists the
recent entries:

```text
1  [dir]   /home/me/projects/pairee
2  [file]  /home/me/projects/pairee/README.md
3  [dir]   /home/me/Downloads
```

Press `2` to open `/home/me/projects/pairee` with `README.md` in it.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| "No recent files tracked yet — visit a few directories first" | The history is empty (fresh install) | Visit a few folders so `on_cd` fills the list. |
| Nothing is being recorded | `record_dirs = false` in the plugin settings | Turn it back on in the Plugin Manager settings of `recent-files`. |
| Files never show up | `record_hover` is off (the default) | Turn `record_hover` on to record the files under the cursor too. |
| Plugin fails to load | It needs Pairee 0.11.0 or newer (Lua API 1.3.0) | Update Pairee, then reinstall with `pairee plugin install recent-files`. |
