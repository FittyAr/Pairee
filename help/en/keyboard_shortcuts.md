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
| `Enter` / `Ctrl+PgDn` | Enter the highlighted folder, or open the highlighted zip / tar / tar.gz / 7z archive as a folder (`Ctrl+PgDn` never opens files). |
| `Backspace` / `Ctrl+PgUp` | Go to the parent folder; at the root of an archive, back to the folder that holds it (cursor on the archive). |
| `Ctrl+U` | Swap directory paths between the Left and Right panels. |
| `Ctrl+H` | Toggle visibility of hidden files and system dotfiles. |
| `Ctrl+R` | Reread and refresh the active panel's directory content (local folders also refresh by themselves when they change, see **Panel** settings). |
| `Ctrl+\` | Open the Directory Hotlist (`Enter` go, `Ins`/`+` add the current folder, `Del`/`-` remove). Saved in `bookmarks.toml`. |
| `Ctrl+Alt+1` … `Ctrl+Alt+9` | Jump to folder shortcut 1–9. Assign them from **Commands → Folder shortcuts** (`Ins`/`Space` or the slot digit assigns the current folder, `Del` clears). |

### Folder tabs

Each panel can hold several folder tabs. The tab bar above a panel appears when it has more than one tab (or always, with **Options → Configuration → Panel → Always show tab bar**); click a tab to show it, middle-click to close it. Locked tabs are marked with `*`. Everything is also in **Left/Right → Tabs**.

| Key / Shortcut | Action |
| :--- | :--- |
| `Alt+T` (`Ctrl+T` in the NeoVim and VSCode keymaps) | New tab on the current folder (in Norton `Ctrl+T` stays the transfer panel). |
| `Alt+O` / `Ctrl+Enter` | Open the folder or archive under the cursor in a new tab (`Ctrl+Enter` needs a terminal that reports it, e.g. kitty, WezTerm, foot). |
| `Alt+W` | Close the current tab (the last tab of a panel stays; `Ctrl+W` is the task list). |
| `Alt+PgDn` / `Alt+Right` | Next tab. |
| `Alt+PgUp` / `Alt+Left` | Previous tab. |
| `Alt+Shift+PgUp` / `Alt+Shift+PgDn` | Move the current tab left / right. |
| `Alt+1` … `Alt+9` | Show tab 1–9 of the focused panel. |
| *(menu / command palette)* | **Rename tab** (empty name: the folder name) and **Lock tab**: a locked tab keeps its folder, and entering another folder from it opens that folder in a new tab. |

`Ctrl+Tab` keeps switching screens. Some terminals use `Alt+arrows` or `Alt+digits` for themselves (Windows Terminal panes, GNOME Terminal tabs); `Alt+PgUp`/`Alt+PgDn` work there, and every chord can be changed in the keymap.

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
| `F3` | Open internal file viewer (supports Hex and Text modes). On a folder: compute its size. |
| `Alt+F3` | Open internal file viewer in alternate mode. |
| `F4` | Open the built-in editor (see [section 7](#-7-built-in-editor-f4)). |
| `F5` | Copy highlighted or selected files to passive panel destination. |
| `Alt+F5` | Print file utility (applies custom filter command). |
| `F6` | Move selected items to passive panel destination. Press `Tab` from the path field to expand advanced options (symlinks, attributes, filter, etc.). |
| `Alt+F6` | Open Symlink / Hardlink creation dialog. |
| `Alt+Backspace` | Undo the last file operation (rename, move, copy, make folder, link, send to trash) after a confirmation (see Features, section 2.4.1). |
| `Ctrl+Y` | Redo the last undone file operation. |
| `F7` | Rename the highlighted item in place (filename only). |
| `Shift+F6` | Multi-rename the selected items with masks, counter, search & replace and a live preview (see Features, section 2.2.1). In the dialog: `Tab`/arrows move between fields, `Space` toggles options, `Left`/`Right` change the case mode, `PgUp`/`PgDn`, the mouse wheel or the scrollbar scroll the preview, `Enter` renames, `Esc` cancels. |
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
| `Insert` / `Space` | Tag/untag active file selection. Cursor automatically moves down. On a folder it also computes the folder size (`Esc` stops it). |
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
| `Ctrl+p` / `Ctrl+P` | Cycle bottom F-key modifier row manually (Normal -> Ctrl -> Alt -> Shift). Each row shows what the keymap binds to `F1`…`F12` with that modifier (e.g. `Shift+F6` multi-rename). |
| `s` | (With Yazi workflow enabled) Open Yazi Sort modal panel. |
| `v` | (With Yazi workflow enabled) Open Yazi View modal panel. |
| `Ctrl+Tab` | Switch focus to next open screen background tab. |
| `Ctrl+Shift+Tab` | Switch focus to previous open screen background tab. |

### Keymap files and `Ctrl+Shift` shortcuts

* In `keymaps/*.toml` and `[custom_bindings]` a shifted letter is written as the uppercase letter: `Ctrl+K` means `Ctrl+Shift+K`. The old spelling `Ctrl+Shift+k` is still accepted and rewritten. `Comma` names the `,` key (`Ctrl+Comma`), since commas separate alternative chords.
* `Ctrl+Shift+<letter>` shortcuts (copy path `Ctrl+Shift+C`, command palette `Ctrl+Shift+P`, key overlay `Ctrl+Shift+K`, SSH `Ctrl+Shift+S`) need a terminal that reports `Shift` together with `Ctrl` (Windows Terminal, kitty, WezTerm, foot…).
* VSCode keymap: `F2` is only Rename; the user menu moved to `Alt+U`, swap panels to `Ctrl+U`, find file to `Ctrl+Shift+F` (`Ctrl+F` is the quick filter), show hidden files to `Ctrl+.`, and settings `Ctrl+,` now works.
* Every shipped keymap loads without errors or duplicate chords (checked by a test).

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
| `Shift` + arrows / `Home` / `End` / `PgUp` / `PgDn` / `Ctrl+Home` / `Ctrl+End` | Select text. Dragging with the mouse also selects. |
| `Alt+Shift` + arrows / `Home` / `End` | Vertical block (column) selection, Far style. `Alt` + mouse drag also selects a block. |
| `Ctrl+A` | Select all. `Esc` clears the selection. |
| `Ctrl+C` / `Ctrl+Insert` | Copy the selection. |
| `Ctrl+X` / `Shift+Delete` | Cut the selection. |
| `Ctrl+V` / `Shift+Insert` | Paste (one undo step). Pasting from the terminal (bracketed paste) inserts every line. |
| Typing, `Backspace`, `Delete` | Replace / delete the selection. |
| `Tab` | Insert a tab, or spaces up to the next tab stop when **Expand tabs** is on. |
| `F4` | Open the file in the viewer (hex mode). |
| `F8` | Discard changes and close. |
| `Esc` / `F10` | Close (asks when there are unsaved changes). |

The editor keeps the file's line endings (LF or CRLF), final newline and UTF-8 BOM. Files that are not valid UTF-8 or larger than 64 MiB are not opened (use the viewer instead).

Copy and cut use the system clipboard. When it is not available (for example over SSH or on a headless Linux session) Pairee keeps the text in its own clipboard, so copy and paste still work between editor screens. A copied block is pasted as ordinary lines. Hold `Shift` to see `Shift+F2` (save as) and `Shift+F7` in the key bar.

---

## 👁️ 8. Internal Viewer (F3)

The viewer reads files page by page, so even multi-gigabyte files open at once; the line count grows in the status line while the file is indexed in the background. The status line (bottom border) shows the encoding, the current line (or hex offset) and the progress of indexing or of a running search.

| Key / Shortcut | Action |
| :--- | :--- |
| `Up` / `Down` / `PgUp` / `PgDn` | Scroll. |
| `Home` / `End` | First / last line (or hex row). |
| `F4` | Toggle Text / Hex (and Image for pictures). |
| `F6` | Open the file in the built-in editor. |
| `F7` | Search (from the current line, wrapping to the top). `F3` repeats the last search. |
| `F8` | Choose the encoding (UTF-8, UTF-16 LE/BE, Windows/ISO code pages, Shift-JIS, EUC, GBK, Big5...). |
| `Esc` | Stop a running search; otherwise close the viewer (also `F10`). |

The encoding is detected automatically: a byte-order mark first (UTF-8, UTF-16 LE/BE), then UTF-16 without a mark, then UTF-8, and otherwise the most likely legacy code page (for example Windows-1252 for Latin-1 accents or Shift-JIS). Files in another encoding are shown as text instead of being treated as binary. Detection can be turned off in **Options → Configuration → Editor/Viewer**.
