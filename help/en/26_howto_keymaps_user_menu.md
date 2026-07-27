# 🔧 How-To: Keymaps and the User Menu

> **Quadrant: HOW-TO** — *problem-oriented.*

Pairee ships **three keymap presets** and lets you remap any key in a
`user.toml` file. The **User Menu** (`F2`) is a separate, lightweight
shortcut layer for the commands you run the most.

---

## Switch keymap preset

The three bundled presets live in the `keymaps/` folder, alongside the
executable (or in `/usr/share/pairee/keymaps/` on Linux):

| Preset | Inspired by | Best for |
| --- | --- | --- |
| `norton.toml` | Norton Commander, Far Manager | Classic dual-panel users. |
| `neovim.toml` | Neovim / Oil.nvim / NvimTree | Modal, `h/j/k/l` lovers. |
| `vscode.toml` | VS Code Explorer | Developers with Ctrl-heavy muscle memory. |

### Pick a preset

1. Open `settings.toml` (located in your Pairee config folder).
2. Set the `keymap` field:

   ```toml
   keymap = "neovim"
   ```

3. Save and restart Pairee. The F-key bar and all keybindings
   immediately reflect the new preset.

> Valid values match the preset filenames without the extension. See
> [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
> for a side-by-side comparison of the three presets.

### Create your own preset

1. Copy any of the bundled `.toml` files to a new file inside the
   `keymaps/` directory.
2. Edit the `[bindings]` table. Each line is `action = "KeyCombo"`.
   Multiple bindings for the same action are allowed (comma-separated
   inside the string).
3. Reference the preset by its filename stem in `settings.toml`:

   ```toml
   keymap = "my_custom"
   ```

For the full action list, see
[`43_reference_actions`](43_reference_actions.md).

### Layer a user keymap

If you only want to **override a few keys** without writing a full
preset, create `keymaps/user.toml`. Keys in `user.toml` take
precedence over the active preset, and you do not have to maintain a
full copy.

---

## The User Menu (F2)

`F2` opens a small overlay with a numbered list of quick commands.
The default items are:

| # | Label | Action |
| --- | --- | --- |
| 1 | Refresh | `Ctrl+R` |
| 2 | Toggle hidden | `Ctrl+H` |
| 3 | Swap panels | `Ctrl+U` |
| 4 | Task list | `Ctrl+W` |
| 5 | Git panel | `Alt+G` |
| 6 | Make folder | (MkDir dialog) |
| F | Quick filter | Live substring filter |
| H | Help overlay | `F1` |
| E | Edit user menu | Opens the editor |

> Press the **highlighted letter** to fire the action. `Esc` closes
> the menu.

### Customise the User Menu

The "Edit user menu" entry (and pressing `E` in the menu) opens a
TOML editor for `usermenu.toml` in your config folder. The file maps a
key to **either a label-action** or a **shell command template**.

```toml
[[items]]
key = "1"
label = "Refresh"
action = "refresh"            # or a custom command instead

[[items]]
key = "2"
label = "Git status (one-liner)"
command = "git -C %p status -sb"
# %p = current panel path, %f = highlighted file, %% = literal %
```

When the menu has at least one `command = "..."` line, Pairee uses
your custom menu in place of the default.

> The `E` (Edit) entry is always appended at the end so you can
> re-open the editor.

---

## Folder shortcuts (Ctrl+Alt+1 … 9)

You can bind up to **nine paths** to `Ctrl+Alt+1` through `Ctrl+Alt+9`.
Setup:

1. Press `Ctrl+\` to open the **Folder shortcuts** dialog.
2. Pick a slot (`Insert` to add a new entry, `e` to edit, `Delete` to
   remove).
3. Type the path. `Enter` to save.

To **jump** to a saved shortcut, press the corresponding `Ctrl+Alt+N`
key.

---

## Where to go next

- Full action list: [`43_reference_actions`](43_reference_actions.md)
- Side-by-side shortcut tables: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- SSH modifier cycling: [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md)
