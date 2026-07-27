# 🔧 How-To: File Operations

> **Quadrant: HOW-TO** — *problem-oriented, assumes you know the basics.*

This page is a recipe collection for the most common file operations.
Each recipe names the goal, lists the exact steps, and points at edge
cases or configuration knobs that may matter.

> The default hotkeys below come from the **Norton Commander** keymap
> preset. If you use **Neovim** or **VSCode**, see
> [`40_reference_keyboard_shortcuts`](40_reference_keyboard_shortcuts.md).

---

## Tag and untag files

**Goal:** mark a contiguous or non-contiguous set of items for a bulk
operation.

| Key | Effect |
| --- | --- |
| `Insert` / `Space` | Toggle the tag on the highlighted file; the cursor moves down. |
| `Gray+` (keypad) | Tag every file matching a glob (`*.log`, `temp_*`). |
| `Gray-` (keypad) | Untag every file matching a glob. |
| `Gray*` (keypad) | Invert the selection of the **entire** panel. |
| `Ctrl+M` | Restore the last bulk selection snapshot. |

> When "Select folders" is enabled in `Configuration → Panel`, your
> glob also matches directories.

---

## Copy files (F5)

**Goal:** copy the highlighted or tagged items from the active panel
into the passive panel.

1. Navigate the **active** panel to the source and tag the files (or
   leave one highlighted to copy just it).
2. Navigate the **passive** panel to the destination.
3. Press `F5`.
4. If a file already exists at the destination, choose one of:
   - **Overwrite** — replace the destination.
   - **Skip** — keep the destination, continue with the next file.
   - **Append** — concatenate (only valid for some file types).
   - **Ask** — Pairee will prompt again per file.
5. The copy runs in a background worker. A **progress popup** shows
   the current file, transfer speed, ETA, and total bytes. You can
   keep using Pairee while it runs; the popup stays on top.

### Symlink options

If the source is a symbolic link, `F5` opens an extra dialog with:

- **Smartly copy** — copy the symlink itself if the destination
  supports symlinks; otherwise, copy the *target's data*.
- **Copy link** — copy the symlink pointer verbatim.
- **Copy target** — resolve the symlink first; copy the data behind it.

### Advanced options

Press `Tab` while the destination field is focused to reveal:

- Filter (a glob applied to source files)
- Preserve attributes (timestamps, permissions)
- Wipe-and-replace (secure overwrite before move)

---

## Move or rename (F6)

**Goal:** move files (or rename in place) between panels.

1. Tag the files (or leave one highlighted).
2. Press `F6`.
3. A dialog asks for the destination path. Confirm.
4. Same progress popup and overwrite handling as `F5`.

`F6` can also be used as a **rename-and-move** for a single file: type
the new path and Pairee moves it to the new location in one step.

---

## Rename a single file in place (F7)

**Goal:** change only the name (not the location) of the highlighted
file.

1. Highlight the file.
2. Press `F7`.
3. Type the new name. `Enter` to confirm, `Esc` to cancel.

> `F7` does **not** move across directories. Use `F6` for that.

---

## Delete (F8 / Delete)

**Goal:** remove the highlighted or tagged items.

1. Tag the files.
2. Press `F8` (or `Delete`).
3. Pairee shows a confirmation dialog. Accept.

### What actually happens

Pairee follows the **Delete to Recycle Bin** setting in
`Configuration → System`:

- **Enabled** (default): items are moved to the OS recycle bin
  (`shell32` on Windows, `trash-cli` / `gio trash` on Linux). They
  can be restored from the system trash.
- **Disabled**: items are deleted permanently with no recovery.

Either way, the operation runs asynchronously.

---

## Secure wipe (Alt+Delete)

**Goal:** overwrite a file's sectors with random data before deletion,
so forensic recovery is impossible.

1. Highlight the file.
2. Press `Alt+Delete`.
3. Confirm. Pairee writes multiple overwrite passes (configurable
   count) of random bytes, then deletes the file.

> Secure wipe is **slow** and works on regular files only. It cannot
> wipe an entire SSD due to wear-levelling, but it does prevent casual
> recovery from the freed sectors.

---

## Create a folder (MkDir)

**Goal:** make a new directory inside the active panel.

1. Press `F9` → `Files` → `Make folder`. Or press `F2` (User Menu) and
   choose the **`6` Make folder** entry.
2. Type the name. `Enter` to confirm, `Esc` to cancel.

> You can also create a chain of folders by typing a path with `/`
> separators; Pairee will create the intermediate directories.

---

## Create a link (Alt+F6)

**Goal:** create a symbolic or hard link to the highlighted file or
folder.

1. Highlight the source.
2. Press `Alt+F6` (or `Files → Link`).
3. Choose **Symbolic** or **Hard** link.
4. Type the destination path.
5. Confirm.

> Hard links cannot cross filesystem boundaries and cannot target
> directories. Symbolic links can do both, but are followed by default
> in copy/move unless you choose otherwise (see F5 symlink options).

---

## View file (F3) and View Alternate (Alt+F3)

**Goal:** read a file without editing it.

- `F3` opens the **internal viewer**. Toggle text/hex with `F4` from
  inside the viewer. Search with `F7`.
- `Alt+F3` opens the **alternate viewer** — same as `F3` but starting
  in the *other* mode (text ↔ hex).

If `Configuration → Editor/Viewer → Use external viewer` is set, `F3`
delegates to an external command (use `%f` for the file path).

---

## Edit file (F4)

**Goal:** modify a text file.

1. Highlight the file.
2. Press `F4`.
3. The internal editor opens in a new screen.

Internal editor hotkeys:

| Key | Effect |
| --- | --- |
| `F2` | **Save** the buffer. |
| `F4` | Toggle text / hex mode. |
| `F7` | Search. |
| `F8` | Discard changes (with confirmation). |
| `F10` | Quit (asks to save if dirty). |

If `Configuration → Editor/Viewer → Use external editor` is set, `F4`
delegates to an external command (use `%f` for the file path).

---

## View and change attributes (Ctrl+A)

**Goal:** inspect or change file metadata.

1. Highlight the file.
2. Press `Ctrl+A` (or `Files → File attributes`).
3. A dialog shows:
   - On Unix: permissions (octal + symbolic), owner, group, mtime, atime.
   - On Windows: read-only, hidden, archive, system flags, timestamps.
4. Toggle flags and `Enter` to apply.

> The same dialog is used to set "Hidden" / "System" on Windows, or to
> chmod a file to `0755` on Linux.

---

## Compare folders (Commands → Compare folders)

**Goal:** diff the two panels.

1. Navigate the left panel to one folder, the right panel to another.
2. `F9` → `Commands` → `Compare folders`.
3. A dialog lists files that are:
   - only on the left
   - only on the right
   - same name, different size or mtime
   - identical
4. Differences are **automatically tagged** in the active panel, so
   you can copy them over with `F5`.

---

## Run a command on the selected files (Ctrl+G)

**Goal:** apply the same shell command to every tagged file.

1. Tag the files.
2. Press `Ctrl+G` (or `Files → Apply command`).
3. Type a template that includes `%f` (filename) and/or `%p` (path).
   Example: `convert %f -resize 50% small_%f` to resize all tagged
   images.
4. The command runs once per file, with the placeholder replaced.

> Use the **%f** / **%p** / **%%** convention. The output streams to
> the panel-strip area while the command runs.

---

## Edit a description (Ctrl+Z)

**Goal:** add a one-line description for the highlighted file.

1. Highlight the file.
2. Press `Ctrl+Z` (or `Files → Describe files`).
3. Type the description. `Enter` to save.
4. Pairee writes it to a `Descript.ion` file in the same directory.
   The `Ctrl+6` view mode will render descriptions next to filenames.

---

## Where to go next

- The async transfer mechanism (why the UI never freezes): [`50_explanation_architecture`](50_explanation_architecture.md)
- Recycle bin, secure-wipe, and admin elevation knobs: [`41_reference_configuration`](41_reference_configuration.md)
