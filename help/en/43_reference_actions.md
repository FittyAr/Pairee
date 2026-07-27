# 📖 Reference: Action Enum and Keymap Schema

> **Quadrant: REFERENCE** — *information-oriented.*

Every keybinding in Pairee is a mapping from a **key string** to an
**action** (a variant of the `Action` enum in `src/keybindings/actions.rs`).
This page documents both sides so you can read or write a custom
keymap preset.

> If you only need a quick way to bind a few keys, see
> [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md) for the
> `user.toml` overlay.

---

## 1. Keymap file format

A preset is a TOML file inside the `keymaps/` directory, named
`<preset_name>.toml`. The structure is:

```toml
# Comments start with `#`

[bindings]
# action_name = "KeyCombo"
# Multiple bindings for the same action are allowed (comma-separated).
move_up   = "Up, k"
move_down = "Down, j"
quit      = "F10, Ctrl+c"
```

The filename stem is the preset name. Select the preset in
`settings.toml`:

```toml
keymap = "my_custom"
```

> Three presets ship with Pairee: `norton`, `neovim`, `vscode`.

---

## 2. Key string syntax

| Part | Examples | Notes |
| --- | --- | --- |
| Letter | `a`, `Z`, `0`, `_` | Single character, exact case. |
| Special | `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`, `Backspace`, `Tab`, `Enter`, `Esc`, `Space`, `Insert`, `Delete`, `Menu` | Use the exact word. |
| Function | `F1` … `F12` | |
| Modifier | `Ctrl`, `Shift`, `Alt` | Joined with `+`. |
| Keypad | `Gray+`, `Gray-`, `Gray*` | `Gray` is the keypad `Num Lock` off prefix used in the original Norton Commander. |

Combine modifiers with `+`:

```text
Ctrl+Shift+S
Alt+F10
Shift+Delete
Ctrl+Alt+1
```

Multiple bindings for the same action are a comma-separated string:

```toml
quit = "F10, Ctrl+c, Ctrl+q"
```

---

## 3. The `Action` enum

The full list, grouped. The comment on each line is the default
Norton preset binding.

### Navigation

| Variant | Default | Effect |
| --- | --- | --- |
| `MoveUp` | `Up` | Move cursor up. |
| `MoveDown` | `Down` | Move cursor down. |
| `PageUp` | `PageUp` | One page up. |
| `PageDown` | `PageDown` | One page down. |
| `GoToTop` | `Home` | Jump to first item. |
| `GoToBottom` | `End` | Jump to last item. |
| `ChangePanel` | `Tab` | Switch panel focus. |
| `SelectItem` | `Insert`, `Space` | Tag / untag. |
| `Execute` | `Enter` | Open or run. |
| `GoParent` | `Backspace` | Go to parent directory. |

### Panel view modes (Ctrl+1 … Ctrl+9)

| Variant | Default | Effect |
| --- | --- | --- |
| `PanelViewBrief` | `Ctrl+1` | Names only, multi-column. |
| `PanelViewMedium` | `Ctrl+2` | Name + extension. |
| `PanelViewFull` | `Ctrl+3` | Name, size, date. |
| `PanelViewWide` | `Ctrl+4` | Wide single column. |
| `PanelViewDetailed` | `Ctrl+5` | Perms, owner, group, links. |
| `PanelViewDescriptions` | `Ctrl+6` | Name + `Descript.ion`. |
| `PanelViewFileOwners` | `Ctrl+7` | Name + owners. |
| `PanelViewFileLinks` | `Ctrl+8` | Name + hardlink count. |
| `PanelViewAltFull` | `Ctrl+9` | User-defined columns. |

### Panel toggles

| Variant | Default | Effect |
| --- | --- | --- |
| `TogglePanelLeft` | `Ctrl+F1` | Show / hide left panel. |
| `TogglePanelRight` | `Ctrl+F2` | Show / hide right panel. |
| `ToggleBothPanels` | `Ctrl+O` | Hide both; press again to restore. |
| `InfoPanel` | `Ctrl+L` | Info overlay. |
| `QuickView` | `Ctrl+Q` | Preview in passive panel. |
| `SortModes` | `Ctrl+F12` | Open the Sort Modes dialog. |

### Sorting (Ctrl+F3 … Ctrl+F11)

| Variant | Default | Effect |
| --- | --- | --- |
| `SortByName` | `Ctrl+F3` | Name. |
| `SortByExtension` | `Ctrl+F4` | Extension. |
| `SortByWriteTime` | `Ctrl+F5` | mtime. |
| `SortBySize` | `Ctrl+F6` | Size. |
| `SortUnsorted` | `Ctrl+F7` | Filesystem order. |
| `SortByCreationTime` | `Ctrl+F8` | Birth time. |
| `SortByAccessTime` | `Ctrl+F9` | atime. |
| `SortByDescription` | `Ctrl+F10` | `Descript.ion`. |
| `SortByOwner` | `Ctrl+F11` | Owner. |
| `ToggleSortReverse` | `Ctrl+Shift+R` | Reverse current sort. |

### F-key actions

| Variant | Default | Effect |
| --- | --- | --- |
| `Help` | `F1` | Open the help popup. |
| `About` | (menu) | About dialog. |
| `UserMenu` | `F2` | Open the User Menu. |
| `View` | `F3` | View file. |
| `ViewAlt` | `Alt+F3` | View in alternate mode. |
| `Edit` | `F4` | Edit file. |
| `Copy` | `F5` | Copy. |
| `Move` | `F6` | Move / rename-and-move. |
| `Rename` | `F7` | Rename in place. |
| `MkDir` | (F2 → 6) | Create folder. |
| `Delete` | `F8` | Delete. |
| `Menu` | `F9` | Open the top menu bar. |
| `Quit` | `F10` | Quit Pairee. |
| `PluginMenu` | (menu) | Open the Plugin Manager. |
| `ScreensList` | `F12` | Screens overlay. |
| `NextScreen` | `Ctrl+Tab` | Next screen. |
| `PrevScreen` | `Ctrl+Shift+Tab` | Previous screen. |

### File operations

| Variant | Default | Effect |
| --- | --- | --- |
| `PrintFile` | `Alt+F5` | Print (applies a custom filter command). |
| `CreateLink` | `Alt+F6` | Symbolic or hard link. |
| `WipeFile` | `Alt+Delete` | Secure overwrite + delete. |
| `FileAttributes` | `Ctrl+A` | Attributes dialog. |
| `ApplyCommand` | `Ctrl+G` | Run a shell command on selected files. |
| `DescribeFile` | `Ctrl+Z` | Edit `Descript.ion` entry. |
| `CompressFiles` | `Shift+F1` | Compress. |
| `ExtractArchive` | `Shift+F2` | Extract. |
| `ArchiveCommands` | `Shift+F3` | Archive sub-menu. |

### Bulk selection

| Variant | Default | Effect |
| --- | --- | --- |
| `SelectGroup` | `Gray+` | Tag by glob. |
| `UnselectGroup` | `Gray-` | Untag by glob. |
| `InvertSelection` | `Gray*` | Invert. |
| `RestoreSelection` | `Ctrl+M` | Restore last selection snapshot. |

### Search & history

| Variant | Default | Effect |
| --- | --- | --- |
| `FindFile` | `Alt+F7` | Find files (name or content). |
| `CommandHistory` | `Alt+F8` | Command-line history. |
| `FileViewHistory` | `Alt+F11` | Files opened with `F3`/`F4`. |
| `FoldersHistory` | `Alt+F12` | Recently visited folders. |

### Commands

| Variant | Default | Effect |
| --- | --- | --- |
| `CompareFolder` | (menu) | Compare the two panels. |
| `EditUserMenu` | (menu) | Open the User Menu editor. |
| `FileAssociations` | (menu) | Open the File Associations editor. |
| `FolderShortcutsConfig` | (menu) | Open the Folder Shortcuts dialog. |
| `FilePanelFilter` | `Ctrl+I` | Set a persistent filter on the active panel. |
| `QuickFilter` | `Ctrl+F` (or `f`/`F`) | Live substring filter. |
| `TaskList` | `Ctrl+W` | OS process list. |

### Options

| Variant | Default | Effect |
| --- | --- | --- |
| `SaveSetup` | `Shift+F9` | Persist all settings immediately. |
| `SystemSettings` | (menu) | Open the Configuration dialog. |
| `CheckForUpdates` | (menu) | Open the update dialog. |

### General

| Variant | Default | Effect |
| --- | --- | --- |
| `ToggleHidden` | `Ctrl+H` | Show / hide hidden files. |
| `FocusCli` | (N/A) | Focus the command line. |
| `Unfocus` | `Esc` | Unfocus / close popup. |
| `Refresh` | `Ctrl+R` | Refresh both panels. |
| `SwapPanels` | `Ctrl+U` | Swap left and right. |
| `DriveSelectLeft` | `Alt+F1` | Drive menu (left). |
| `DriveSelectRight` | `Alt+F2` | Drive menu (right). |
| `ContextMenu` | `Menu`, `Alt+M` | Context menu. |
| `GoFolderShortcut(u8)` | `Ctrl+Alt+1` … `Ctrl+Alt+9` | Jump to a saved folder shortcut. |
| `ToggleLongNames` | `Ctrl+N` | Toggle long-name rendering. |
| `RereadPanel` | `Ctrl+R` | Re-read the active panel. |
| `VideoMode` | `Alt+F9` | Show a video-mode hint dialog. |
| `TreeView` | `Alt+F10` | Tree overlay. |
| `CycleFKeysModifiers` | `Ctrl+P` | Cycle the F-key bar (Normal/Ctrl/Alt). |
| `SshConnect` | `Ctrl+Shift+S` | SSH connection dialog. |
| `SshDisconnect` | (menu) | Disconnect the active panel. |
| `OpenGitPanel` | `Alt+G` | Open the Git dashboard. |
| `InstallDevPlugin` | (dev mode) | Install a local dev plugin. |
| `ToggleTransferPanel` | `Ctrl+T` | Show / hide the Transfer Panel. |

---

## 4. Wildcards / multiple keys

The same `Action` can be bound to multiple keys. Pairee fires the
action on the **first matching** key. This is also how the F-key bar
shows the binding currently in effect.

```toml
[bindings]
quit = "F10, Ctrl+c, Ctrl+q"
```

---

## 5. Unbinding a key

There is no explicit "unbind" syntax. To disable a binding, copy the
preset into a custom file and **remove** the line.

---

## 6. The `user.toml` overlay

If you only want to override a few keys without copying a full
preset, drop your changes in `keymaps/user.toml`. Entries in
`user.toml` are merged on top of the active preset; the active preset
wins for keys you do not override.

---

## Where to go next

- All three preset bindings side by side: [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md)
- Switching presets: [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md)
