# 📖 Reference: Keyboard Shortcuts

> **Quadrant: REFERENCE** — *information-oriented. Look things up.*

This page lists every keybinding in every bundled preset. For *how* to
switch presets, see [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md).
For the full list of *actions* (the second column), see
[`43_reference_actions`](43_reference_actions.md).

> **Conventions**
> - `+` means "and" (`Ctrl+Shift+S` = hold Ctrl, hold Shift, press S).
> - `,` means "alternative binding" (e.g. `F2` and `Insert` both fire
>   `select_item` in the Neovim preset).
> - The first column is the preset; the same action can have different
>   keys in different presets.

---

## 1. Navigation

| Action | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Move cursor up | `Up` | `k`, `Up` | `Up` |
| Move cursor down | `Down` | `j`, `Down` | `Down` |
| Page up | `PageUp` | `Ctrl+u` | `PageUp` |
| Page down | `PageDown` | `Ctrl+d` | `PageDown` |
| Go to top | `Home` | `g`, `Home` | `Home` |
| Go to bottom | `End` | `G`, `End` | `End` |
| Change panel | `Tab` | `Tab` | `Tab` |
| Tag / untag | `Insert`, `Space` | `v`, `Insert`, `Space` | `Space`, `Insert` |
| Execute (Enter) | `Enter` | `l`, `Enter` | `Enter` |
| Go to parent | `Backspace` | `h`, `Backspace` | `Backspace` |

---

## 2. Panel view modes (Ctrl+1 … Ctrl+9)

Identical across all three presets:

| Key | Mode |
| --- | --- |
| `Ctrl+1` | Brief (names only) |
| `Ctrl+2` | Medium (name + ext) |
| `Ctrl+3` | Full (name, size, date) |
| `Ctrl+4` | Wide (wide single column) |
| `Ctrl+5` | Detailed (perms, owner, group, links) |
| `Ctrl+6` | Descriptions (`Descript.ion`) |
| `Ctrl+7` | File owners |
| `Ctrl+8` | File links |
| `Ctrl+9` | Alt Full (user-defined columns) |

---

## 3. Panel toggles

| Action | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Toggle left panel | `Ctrl+F1` | `Ctrl+F1` | `Ctrl+F1` |
| Toggle right panel | `Ctrl+F2` | `Ctrl+F2` | `Ctrl+F2` |
| Toggle both panels | `Ctrl+O` | `Ctrl+O` | `Ctrl+B` |
| Info panel | `Ctrl+L` | `Ctrl+L` | `Ctrl+L` |
| Quick view | `Ctrl+Q` | `Ctrl+Q` | `Ctrl+Shift+Q` |
| Toggle long names | `Ctrl+N` | `Ctrl+N` | (—) |
| Swap panels | `Ctrl+U` | `Ctrl+S` | `Ctrl+Shift+S` |
| Refresh | `Ctrl+R` | `Ctrl+R` | `Ctrl+Shift+E` |
| Toggle hidden | `Ctrl+H` | `Ctrl+H` | `Ctrl+Shift+.` |
| Save setup | `Shift+F9` | `Shift+F9` | `Ctrl+S` |
| Cycle F-key modifiers | `Ctrl+P` | `Ctrl+P` | `Ctrl+P` |

---

## 4. Sorting (Ctrl+F3 … Ctrl+F12)

Identical across all three presets:

| Key | Sort |
| --- | --- |
| `Ctrl+F3` | Name |
| `Ctrl+F4` | Extension |
| `Ctrl+F5` | Write time |
| `Ctrl+F6` | Size |
| `Ctrl+F7` | Unsorted |
| `Ctrl+F8` | Creation time |
| `Ctrl+F9` | Access time |
| `Ctrl+F10` | Description |
| `Ctrl+F11` | Owner |
| `Ctrl+F12` | Open Sort Modes dialog |

---

## 5. File actions (F-keys + Alt/Shift)

| Action | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Help | `F1` | `F1` | `F1` |
| User Menu | `F2` | `F2` | `F2` |
| View | `F3` | `o`, `F3` | `Ctrl+Shift+V`, `F3` |
| Edit | `F4` | `e`, `F4` | `Ctrl+E`, `F4` |
| Copy | `F5` | `y`, `F5` | `Ctrl+C`, `F5` |
| Move / Rename+Move | `F6` | `m`, `F6` | `Ctrl+X`, `F6` |
| Rename in place | `F7` | `r`, `F7` | `F2`, `F7` |
| Make folder | (F2 → `6`) | `Ctrl+Shift+N` | `Ctrl+Shift+N` |
| Delete | `F8` | `d`, `F8` | `Delete`, `F8` |
| Quit | `F10` | `Ctrl+C`, `F10` | `Ctrl+Q`, `F10` |
| Screens list | `F12` | `F12` | `F12` |
| Next screen | `Ctrl+Tab` | `Ctrl+Tab` | `Ctrl+Tab` |
| Previous screen | `Ctrl+Shift+Tab` | `Ctrl+Shift+Tab` | `Ctrl+Shift+Tab` |
| View alternate | `Alt+F3` | `Alt+F3` | (—) |
| Print file | `Alt+F5` | `Alt+F5` | (—) |
| Create link | `Alt+F6` | `Alt+F6` | (—) |
| Secure wipe | `Alt+Delete` | `Alt+Delete` | `Shift+Delete` |
| File attributes | `Ctrl+A` | `Ctrl+A` | `Ctrl+A` |
| Apply command | `Ctrl+G` | `Ctrl+G` | (—) |
| Describe file | `Ctrl+Z` | `Ctrl+Z` | (—) |
| Compress | `Shift+F1` | `Shift+F1` | `Shift+F1` |
| Extract | `Shift+F2` | `Shift+F2` | `Shift+F2` |
| Archive commands | `Shift+F3` | `Shift+F3` | `Shift+F3` |

---

## 6. Search, history, tree

| Action | Norton | Neovim | VSCode |
| --- | --- | --- | --- |
| Find file | `Alt+F7` | `/`, `Alt+F7` | `Ctrl+F` |
| Command history | `Alt+F8` | (—) | `Ctrl+Shift+H` |
| Video mode (info popup) | `Alt+F9` | `Alt+F9` | (—) |
| Tree view | `Alt+F10` | `Alt+F10` | `Ctrl+Shift+T` |
| File view history | `Alt+F11` | `Alt+F11` | `Alt+F11` |
| Folders history | `Alt+F12` | `Alt+F12` | `Alt+F12` |
| File panel filter | `Ctrl+I` | `Ctrl+I` | `Ctrl+I` |
| Quick filter | `Ctrl+F`, `f`, `F` | `Ctrl+F`, `f`, `F` | `Ctrl+F`, `f`, `F` |
| Task list (OS processes) | `Ctrl+W` | `Ctrl+W` | `Ctrl+W` |
| Context menu | `Menu`, `Alt+M` | `Menu`, `Alt+M` | `Menu`, `Shift+F10` |
| SSH connect | `Ctrl+Shift+S` | `Ctrl+Shift+S` | `Ctrl+Shift+S` |
| Git panel | `Alt+G` | `Alt+G` | `Alt+G` |
| Transfer panel | `Ctrl+T` | `Ctrl+T` | `Ctrl+T` |
| Toggle reverse sort | `Ctrl+Shift+R` | (—) | (—) |
| Cycle F-key modifier | `Ctrl+P` | `Ctrl+P` | `Ctrl+P` |
| Folder shortcut 1…9 | `Ctrl+Alt+1` … `Ctrl+Alt+9` | `Ctrl+Alt+1` … `Ctrl+Alt+9` | `Ctrl+Alt+1` … `Ctrl+Alt+9` |

---

## 7. Bulk selection

Identical across all three presets:

| Action | Key |
| --- | --- |
| Select group (glob) | `Gray+` (keypad) |
| Unselect group (glob) | `Gray-` (keypad) |
| Invert selection | `Gray*` (keypad) |
| Restore last selection | `Ctrl+M` |

---

## 8. Top menu bar (F9)

| Menu | Submenu items |
| --- | --- |
| `Left` / `Right` | View mode, Info panel, Quick view, Sort modes, Sort by…, Show long names, Panel on/off, Re-read, Change drive, Connect SSH, Disconnect SSH, Git (auto-shown if path is in a repo) |
| `Files` | View, View alt, Edit, Copy, Print, Rename/Move, Rename, Link, Make folder, Delete, Wipe, Add to archive, Extract files, Archive commands, File attributes, Apply command, Describe files, Select group, Unselect group, Invert selection, Restore selection, Plugin commands, Exit |
| `Commands` | Find file, History, Video mode, Tree view, File view hist, Folders hist, Swap panels, Panels on/off, Compare folders, User menu, Edit user menu, File associations, Folder shortcuts, File panel filter, Screens list, Task list, Hotplug devices, *(if dev mode: Install dev plugin)* |
| `Options` | Configuration…, Check for updates |
| `Help` | About…, *(this help popup is `F1`, not from the menu)* |

---

## 9. The F-key bar (modifiers)

The bottom F-key bar has **four views**. Press `Ctrl+P` to cycle.

| Slot | Default | Ctrl | Alt | Shift |
| --- | --- | --- | --- | --- |
| F1 | Help | Left | Left | — |
| F2 | User | Right | Right | — |
| F3 | View | Name | View | Compress |
| F4 | Edit | Extens | Edit | Extract |
| F5 | Copy | Time | Print | (—) |
| F6 | Move | Size | MkLink | (—) |
| F7 | Rename | Unsort | Find | (—) |
| F8 | Delete | Creatn | History | (—) |
| F9 | Menu | Access | Video | Save |
| F10 | Quit | Descr | Tree | (—) |
| F11 | (—) | Owner | ViewHist | Install dev plugin (if dev mode) |
| F12 | Screen | Sort | FoldHist | (—) |

> Slots with `—` are unbound. They still display the slot number; the
> F-key label is just blank.

---

## 10. In the editor (F4) and viewer (F3) screens

| Action | Key |
| --- | --- |
| Help | `F1` |
| Save (editor only) | `F2` |
| Toggle text / hex | `F4` |
| Search | `F7` |
| Discard changes (editor only) | `F8` |
| Quit | `F10` |

---

## 11. Inside popups

| Popup | Keys |
| --- | --- |
| **Screens list** (`F12`) | `Up` / `Down`, `Enter`, `Esc` |
| **Help** (`F1`) | List: `Up` / `Down`, `Enter` to open, `Tab` or `Left` / `Right` to switch tab. Reader: `Up` / `Down` (or `j` / `k`), `PageUp` / `PageDown`, `Backspace` to return to list, `Esc` to close. |
| **File associations** | `Up` / `Down`, `A` to add, `E` / `Enter` to edit, `D` / `Delete` to remove, `Esc`. |
| **Sort modes** | `Up` / `Down`, `Space` to toggle, `Enter` to apply, `Esc`. |
| **Configuration** | `Tab` to switch tab, `Up` / `Down` to move, `Space` / `Enter` to toggle / edit, `F9` to save, `Esc` to cancel. |

---

## Where to go next

- Full action enum: [`43_reference_actions`](43_reference_actions.md)
- Switch or write a keymap: [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md)
