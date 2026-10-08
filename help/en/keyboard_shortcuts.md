# Keyboard Shortcuts Guide

This guide compiles all interactive hotkeys and keyboard shortcuts available in Pairee, categorized by functionality.

---

## 📂 1. Panel Navigation & Focus

| Key / Shortcut | Action |
| :--- | :--- |
| `Tab` | Switch focus between the active and passive panels. |
| `Up` / `Down` | Move cursor selection highlight up or down. |
| `PageUp` / `PageDown` | Scroll panel list up or down by one screen. |
| `Home` / `End` | Jump directly to the first or last item in the list. |
| `Ctrl+U` | Swap directory paths between the Left and Right panels. |
| `Ctrl+H` | Toggle visibility of hidden files and system dotfiles. |
| `Ctrl+R` | Reread and refresh the active panel's directory content. |
| `Ctrl+\` | Open the Directory Hotlist (`Enter` go, `Ins`/`+` add the current folder, `Del`/`-` remove). Saved in `bookmarks.toml`. |
| `Ctrl+Alt+1` … `Ctrl+Alt+9` | Jump to folder shortcut 1–9. Assign them from **Commands → Folder shortcuts** (`Ins`/`Space` or the slot digit assigns the current folder, `Del` clears). |

---

## 🗂️ 2. Panel View Modes & Toggles

| Key / Shortcut | Action |
| :--- | :--- |
| `Ctrl+1` | **Brief view:** File names only, multi-column. |
| `Ctrl+2` | **Medium view:** File name and extension side-by-side. |
| `Ctrl+3` | **Full view:** Standard details (Name, Size, Date). |
| `Ctrl+4` | **Wide view:** Wide filename listing. |
| `Ctrl+5` | **Detailed view:** Unix permissions, owner, hardlink counts. |
| `Ctrl+6` | **Descriptions view:** Render names alongside `Descript.ion` logs. |
| `Ctrl+7` | **File owners view:** Render usernames and user groups. |
| `Ctrl+8` | **File links view:** Render physical hardlink count numbers. |
| `Ctrl+9` | **Alt Full view:** User-defined customizable columns. |
| `Ctrl+F1` | Show / Hide the Left Panel. |
| `Ctrl+F2` | Show / Hide the Right Panel. |
| `Ctrl+O` | Hide / Show both panels (useful to inspect background CLI output). |
| `Ctrl+Q` | Toggle **Quick View** preview panel on the passive side. |
| `Ctrl+L` | Toggle **System Info** overlay panel on the active side. |

---

## 🛠️ 3. Standard File Actions (F-keys)

| Key | Action |
| :--- | :--- |
| `F1` | Open in-app Help and manuals reader dialog. |
| `F2` | Open custom User Command Menu overlay. |
| `F3` | Open internal file viewer (supports Hex and Text modes). |
| `Alt+F3` | Open internal file viewer in alternate mode. |
| `F4` | Open the built-in editor (see [section 7](#-7-built-in-editor-f4)). |
| `F5` | Copy highlighted or selected files to passive panel destination. |
| `Alt+F5` | Print file utility (applies custom filter command). |
| `F6` | Move selected items to passive panel destination. Press `Tab` from the path field to expand advanced options (symlinks, attributes, filter, etc.). |
| `Alt+F6` | Open Symlink / Hardlink creation dialog. |
| `F7` | Rename the highlighted item in place (filename only). |
| `F8` / `Delete` | Delete highlighted or tagged files. |
| `Alt+Delete` | Secure Wipe: Overwrites file sectors before permanent deletion. |
| `F9` | Pull down and activate the Top Menu Bar. |
| `F10` | Close / Quit Pairee. |
| `F12` | Open screens and background tabs navigation overlay. |
| `Esc` | Clear command prompts, close dialogs, or exit overlays. |
| `Menu` / `Alt+M` | Open context actions menu relative to file type. |

> **Tip:** Items previously on `F7` (MkDir) and `F11` (Plugins) are now reachable from the **Top Menu Bar** → **Files** submenu. **MkDir** is also available as a default entry (`6`) inside the **User Menu** (`F2`).

---

## 🏷️ 4. Tagging & Bulk Selection

| Key | Action |
| :--- | :--- |
| `Insert` / `Space` | Tag/untag active file selection. Cursor automatically moves down. |
| `+` (Keypad) | Tag a group of files matching a glob mask (e.g. `*.rs`). |
| `-` (Keypad) | Untag a group of files matching a glob mask. |
| `*` (Keypad) | Invert selection state of the entire panel. |
| `Ctrl+M` | Restore the last tagged selection group. |
| `Ctrl+I` | Apply a persistent file name search filter on the active panel. |

> **Keymap TOML:** Far-style `Gray+` / `Gray-` / `Gray*` are accepted and mapped to `Plus` / `-` / `*`. Prefer `Plus` in new keymap files. Invalid or duplicate chords are rejected; Settings → Interface lists them.

> **Terminal limitation (`Ctrl+H`, `Ctrl+I`, `Ctrl+M`):** legacy terminals send the same bytes for `Ctrl+H` and `Backspace`, `Ctrl+I` and `Tab`, `Ctrl+M` and `Enter`. These three shortcuts only work in terminals that support the kitty keyboard protocol (kitty, WezTerm, foot, Ghostty, Alacritty, recent Windows Terminal, …). Elsewhere, rebind them in `keybindings.toml`:
>
> ```toml
> [custom_bindings]
> toggle_hidden     = "Alt+h"
> file_panel_filter = "Alt+i"
> restore_selection = "Alt+r"
> ```

---

## 🌐 5. SSH, Git & Task Management

| Key / Shortcut | Action |
| :--- | :--- |
| `Ctrl+Shift+S` | Launch SSH / SFTP connection dialog. |
| `Alt+G` | Launch Git Integration Panel. |
| `Ctrl+W` | Open OS Task List (running system processes table). |
| `Alt+F10` | Open graphical directory Tree View. |
| `Ctrl+p` / `Ctrl+P` | Cycle bottom F-key modifier row manually (Normal -> Ctrl -> Alt). |
| `s` | (With Yazi workflow enabled) Open Yazi Sort modal panel. |
| `v` | (With Yazi workflow enabled) Open Yazi View modal panel. |
| `Ctrl+Tab` | Switch focus to next open screen background tab. |
| `Ctrl+Shift+Tab` | Switch focus to previous open screen background tab. |

---

## 🔗 6. File Associations Editor

| Key / Shortcut | Action |
| :--- | :--- |
| `Up` / `Down` | Navigate through rules list. |
| `A` / `a` / `Insert` | Add a new file association rule. |
| `E` / `e` / `Enter` | Edit the highlighted association rule. |
| `D` / `d` / `Delete` | Delete the highlighted association rule. |
| `Esc` | Close the editor or cancel current editing step. |

---

## 📝 7. Built-in Editor (F4)

Pairee always edits files with its built-in editor; no external editor is launched. It opens from `F4` in a panel, from `F6` in the viewer and from **Commands → Edit user menu**.

| Key / Shortcut | Action |
| :--- | :--- |
| `F2` / `Ctrl+S` | Save. If another program changed the file since it was opened, you are asked before it is overwritten. |
| `Shift+F2` | Save as (a relative name is resolved against the file's folder; an existing file is only overwritten after confirmation). |
| `Ctrl+Z` | Undo (typing runs are undone as one step). |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo. |
| `F7` / `Ctrl+F` | Search. `F3` / `Shift+F7` repeat the last search. |
| `Ctrl+R` | Reload the file from disk (asks first when there are unsaved changes and the confirmation is enabled). |
| `Home` / `End` | Start / end of line. `Ctrl+Home` / `Ctrl+End` go to the start / end of the file. |
| `Tab` | Insert a tab, or spaces up to the next tab stop when **Expand tabs** is on. |
| `F4` | Open the file in the viewer (hex mode). |
| `F8` | Discard changes and close. |
| `Esc` / `F10` | Close (asks when there are unsaved changes). |

The editor keeps the file's line endings (LF or CRLF), final newline and UTF-8 BOM. Files that are not valid UTF-8 or larger than 64 MiB are not opened (use the viewer instead).
