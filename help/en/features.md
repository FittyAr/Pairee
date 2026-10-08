# Pairee Features Reference Manual

This manual provides a detailed description of the core interactive features, utilities, and integrations available in **Pairee**.

---

## 🖥️ 1. Panel Views & Custom Layouts

Pairee utilizes a classic dual-panel layout for folder navigation and file management, designed to keep both directories visible side-by-side.

### 1.1 Panel Display Modes
You can configure each panel independently to display files using different detail levels:
* **Brief:** Displays file names only across multiple columns. Ideal for navigating directories with thousands of files.
* **Medium:** Lists file name and file extension side-by-side.
* **Full / Detailed:** Displays comprehensive filesystem metadata: Name, Extension, Size, Date Modified, Permissions (Unix octals), Owner, and hardlink counts.
* **Wide:** Broad name listing with minimal details.
* **Descriptions:** Renders file name along with description details loaded from `Descript.ion` lists.
* **FileOwners:** Lists files alongside user/group names.
* **FileLinks:** Lists files with hardlink count columns.
* **AltFull:** User-configurable custom column structure.

### 1.2 Panel Visibility & Swapping
* **Toggle Left/Right Panel:** Individually show or hide the left or right panels to focus on a single directory path.
* **Toggle Both Panels:** Hides both panels to inspect the terminal outputs of background commands or previous process executions.
* **Swap Panels:** Instantly swap the paths of the left and right panels.
* **Navigation History:** Displays a popup listing recently visited directories. Select a row and press `Enter` to jump directly.
* **Directory Hotlist:** A custom bookmarks list for adding, deleting, and selecting your most visited folders.

---

## 📂 2. File System Operations

File operations in Pairee are asynchronous, running on a background worker queue (`tokio`) to ensure the user interface remains completely responsive.

### 2.1 Bulk Selection & Tagging
* Tag files by pressing `Insert` or `Space` on an item. The cursor automatically moves down. On a folder this also measures its size (see **Folder sizes** below).
* Use `+` (Keypad) to tag files using wildcard patterns (e.g. `*.rs` or `temp_*`).
* Use `-` (Keypad) to untag files using wildcard patterns.
* Use `*` (Keypad) to invert the selection state of the entire panel.
* **File Panel Filter:** Apply an active glob filter (e.g., `*.rs`) to restrict visible items in the current panel list.

### 2.2 Copy & Move/Rename
* **Background Processing:** Both copy and move tasks run asynchronously, showing real-time progress bars, byte transfer counts, file names, and percentages.
* **Overwrite Resolution:** If a file exists at the target path, Pairee prompts with Ask (prompt dialogue), Overwrite, Skip, or Append.
* **Symbolic Links Options:**
  - *Smartly copy:* Copies the symlink pointer if the destination supports it; otherwise, copies the physical target data.
  - *Copy link:* Copies the symlink pointer itself.
  - *Copy target:* Resolves the symlink and copies the target data.

### 2.2.1 Multi-Rename Tool
Press `Shift+F6` (or **Files → Multi-Rename**) to rename every selected item (or the one under the cursor) in one go. The dialog shows a live preview table (old name, new name, status) that updates while you type; nothing is renamed until you press **Rename**.
* **Name and extension masks:** `[N]` name without extension, `[N2-5]` characters 2 to 5, `[N3]` the 3rd character, `[N2-]` from the 2nd to the end, `[N2,3]` 3 characters from the 2nd, `[E]` extension (same ranges), `[P]` parent folder name, `[C]` counter, `[Y]` `[M]` `[D]` `[h]` `[m]` `[s]` modification date and time. Any other text is copied as is; an empty extension mask drops the dot.
* **Counter:** start value, step (may be negative) and minimum number of digits (zero-padded).
* **Search & replace:** applied to the whole new name; plain text by default, or a regular expression (`$1`, `${name}` in the replacement) when *Regular expression* is ticked. *Ignore case* works for both.
* **Case:** unchanged, lower case, UPPER CASE or Title Case, applied last.
* **Conflict detection:** rows with an empty name, characters the filesystem refuses (`<>:"/\|?*`, reserved names such as `CON` on Windows), duplicate names or names of other files in the folder are shown in red, and **Rename** stays disabled until they are fixed.
* **Safe execution:** renames are ordered so no file is overwritten; swaps and cycles (`a → b`, `b → a`) go through a temporary name. If a rename fails, the ones already done are undone. Works on local and SSH/SFTP panels.

### 2.3 Secure Wipe & Deletion
* **Normal Delete:** Moves files/folders to the system recycle bin or deletes them permanently depending on your settings.
* **Secure Wipe:** Overwrites file sectors with random byte buffers before removal, rendering the data completely unrecoverable by forensic tools.

### 2.4 Creating Links
* Easily create symbolic links or hard links mapping a source file or directory to a specific destination path.

### 2.5 Elevated Privilege Support (Sudo / Admin)
* When a filesystem operation (delete, copy, move, mkdir) encounters a "Permission Denied" error, Pairee prompts you to retry with administrative privileges. It executes the action using an elevated helper process (`sudo` on Unix/Linux, UAC prompt on Windows) without needing to restart the application.

---

## 🔍 3. Search, Viewer, & Editor

### 3.1 Advanced Search
* **Filters:** Search folders recursively using wildcard name masks (e.g., `*.toml`, `src*`).
* **Content Search:** Search for specific text strings inside files.
* **Result Navigation:** The search results popup lists all matches. Select any match and press `Enter` to close the search and jump directly to that file in the active panel.

### 3.2 Internal Viewer & Quick View
* **Viewer Modes:** Toggle between plain Text mode and Hex Dump mode.
* **Hex Dump View:** Displays offsets, hex values, and ASCII representation side-by-side. Excellent for inspecting binary files.
* **Viewer Search:** Press `F7` inside the viewer to search for text strings. The search runs in the background over the whole file (progress in the status line, `Esc` cancels).
* **Large Files:** Files are read page by page with a line index built in the background, so multi-gigabyte files open instantly with bounded memory.
* **Encodings:** The encoding is detected (byte-order mark, UTF-16, UTF-8, legacy code pages such as Windows-1252 or Shift-JIS) and shown in the status line; `F8` switches it manually.
* **Quick View:** Instantly displays a preview of the highlighted file in the opposite panel. Supports text file previews (first 256 KiB, in the detected encoding) and archive metadata listing.

### 3.3 Built-in Editor
* All editing happens in the built-in editor (`F4`); Pairee never launches an external editor.
* Undo/redo (`Ctrl+Z` / `Ctrl+Y`), search, save and save as (`Shift+F2`).
* Selection with `Shift` + movement keys or the mouse, vertical block selection with `Alt+Shift` + arrows, and copy / cut / paste (`Ctrl+C` / `Ctrl+X` / `Ctrl+V`) through the system clipboard, with an internal clipboard when none is available.
* Keeps line endings (LF/CRLF), the final newline and the UTF-8 BOM; honors the tab size, tab expansion, auto indent and line number settings.
* Warns about or locks read-only files, and asks before overwriting a file that another program changed.

---

## 🛠️ 4. In-App Screen Management (Multitasking)

Pairee features a robust multitasking screens architecture. You can spawn several work environments concurrently (e.g., editing one file, viewing another, running a terminal execution, and browsing file panels).

* **Screens List Overlay:** View a list of all active open screens. The active screen is marked with an asterisk (`*`).
* **Suspend/Resume Popups:** Switching screens preserves the state of active popup dialogues. For example, if you are midway through a copy prompt dialog, you can open the Screens Menu, check another file in the Editor, and switch back to resume the copy prompt dialog exactly where you left off.
* **Cycle Shortcuts:** Use hotkeys to cycle forward or backward through your open screen contexts without opening the menu.

---

## 🧰 5. Utilities & Advanced Tools

* **Context Actions Menu:** Opens a popup menu containing actions (View, Edit, Copy, Move, Delete, Compress, Extract) relative to the highlighted file type. Detects archives (ZIP, 7z, RAR, TAR, GZ, BZ2, XZ) and adds dynamic Archive Commands.
* **Folder Compare:** (**Commands → Compare folders**) Compares the left and right panel folders recursively in the background (`Esc` cancels) to identify files that are present in only one panel or differ in size/modification date, highlighting and tagging them; a folder present on both sides is listed as different when anything inside it differs. Modification times up to `compare_mtime_tolerance_secs` apart (2 s by default, the FAT granularity) count as equal; on Windows and macOS names are matched ignoring case. Only local folders can be compared.
* **Synchronize Folders:** (**Commands → Synchronize folders**) Total Commander-style directory synchronization between the two panels:
  1. *Options:* direction (`Space` cycles) — **Left → Right** (copy new and changed files to the right; a newer file on the right is not overwritten by default), **Right → Left**, **Both ways** (the newer file wins; same-time conflicts are skipped) or **Mirror Left → Right** (also deletes what exists only on the right); compare file contents with the transfer hash algorithm (same-size files are then equal only when their contents match); ignore hidden files; and a filter mask in the copy-filter syntax (`*.rs;*.toml` include, `!target` exclude, `;`-separated). Swap the panels (`Ctrl+U`) to mirror the other way.
  2. *Review:* after the comparison (background, `Esc` cancels) every difference is listed with its status, size and planned action, plus totals of files and bytes to copy or delete. Keys: `→`/`>` copy to the right, `←`/`<` copy to the left, `Del`/`D` delete (items on one side only), `S` skip, `Space` cycles the allowed actions, `Tab` changes the direction (resetting the default actions), `E` shows/hides equal files, `Enter` applies, `Esc` goes back to the options.
  3. *Apply:* plans that delete anything ask for an explicit confirmation (`Y`/`Enter`, `N`/`Esc`). The copies and deletes run as ordinary Transfer Engine jobs (progress, pause, cancel and log in the transfer panel); copies overwrite the target and keep the source modification time, and deletions follow the "delete to recycle bin" setting. Only local folders can be synchronized.
* **Folder sizes:** `Space` or `F3` on a folder, or **Commands → Folder sizes** (selected folders, or every folder when none is selected), computes the folder's total size in the background. The size column shows it until you change directory; `Esc` stops the calculation. Symbolic links are not followed, hard links are counted once (Linux/macOS) and folders that could not be read completely are marked with `+`. Works on local and SFTP panels.
* **Disk usage view:** **Commands → Disk usage** scans the current folder (cancellable with `Esc`, with live progress) and lists its contents largest first with a percentage and a bar. `Enter`/`→` opens a subfolder, `←`/`Backspace` goes back, `Del`/`F8` deletes the highlighted item through the regular delete confirmation, `r`/`F5` rescans. The result is cached: reopening the view on the same folder is instant.
* **OS Task Manager:** Displays a table of active system processes with PIDs, names, and memory consumption. Allows process termination using `Delete` or `Alt+Delete`.
* **Directory Tree View:** Traverses the directory structure and displays a graph-like tree layout.
* **File Descriptions:** Supports editing and saving file description tags to hidden `Descript.ion` lists.
* **File Associations:** Map file extensions (e.g., `*.py`) to custom launch commands.
* **Custom User Commands Menu:** Define custom shell commands or script execution shortcuts to run on highlighted or tagged files. The popup is bound to `F2` and ships with a small set of built-in shortcuts (Refresh, Toggle hidden, Swap panels, Task list, Git panel, **Create folder**, Quick filter, Help, Edit).
* **Drive Select Panel:** Displays removable disks, external USB drives, and mounted network drives to switch panel paths.
* **System Info Panel:** Overlay window displaying current OS distribution name, machine hostname, logged-in username, available system RAM, and environment parameters.

---

## 🌐 6. Smart Auto-Update System

Pairee features a fully integrated auto-update system that determines how your application was installed and processes updates securely and automatically.

### 6.1 Interactive Notification & Releases Popup
* **Automatic Checks:** If enabled, Pairee checks the latest GitHub releases in the background at startup.
* **Update Badge:** If a new release is available, a yellow `▲ UPDATE` indicator badge appears at the top right of the screen (next to the clock).
* **Release Notes Viewer:** Clicking on the badge or selecting `Check for updates` from the `F9 (Options)` menu opens the Update dialog. This popup fetches and formats the release notes / changelog directly from GitHub and shows the release size.

### 6.2 Platform & Package-Manager Actions
Pairee checks 13 different installation paths to apply updates correctly:
* **Direct Binaries:**
  - **Linux (tar.gz):** Downloads and performs an atomic binary replacement in the active run path. A restart is prompted.
  - **Windows (ZIP):** Downloads the release, writes a temporary self-destructing batch script helper, and updates the executable cleanly after Pairee exits.
  - **Windows (Inno Setup):** Downloads the installer and executes it silently in the background (`/VERYSILENT`).
* **Package Managers:** If Pairee detects it was installed via a package manager (e.g., `apt`, `dnf`/`rpm`, `pacman`, `nix`, `snap`, `flatpak` on Linux, or `winget`, `scoop`, `chocolatey` on Windows), it displays the exact console command required to update Pairee (e.g., `winget upgrade Pairee` or `sudo apt update && sudo apt install pairee`). You can easily view this command to run it in your shell.

### 6.3 Secure Signature Verification
To prevent running compromised binaries, Pairee's built-in downloader automatically fetches the corresponding `.sha256` hash from GitHub Releases and validates the downloaded payload's integrity before initiating any installation.

---

## 🧩 7. Plugin System & Developer Tools

Pairee supports a Lua-based plugin system that allows extending the file manager with custom commands, file previewers, and lifecycle hooks.

### 7.1 Plugin Manager

Open the Plugin Manager via **Top Menu Bar → Files → Plugin commands**. `F11` is no longer bound to this action in the default keymap. It has three tabs:

- **Installed:** Lists all loaded plugins with version, trust badge, and available updates. Use `Enter` to toggle trust/pin, `D` to uninstall.
- **Registry:** Search the online plugin registry and install plugins in the background.
- **Developer Tools:** Available when `plugins_developer_mode = true` in settings. Provides the initialization wizard, lint, package, and submit tools.

### 7.2 Initializing a New Plugin

In the **Developer Tools** tab, select **Initialize New Plugin** and follow the step-by-step wizard:
1. Enter the plugin **name** (used as folder name and manifest identifier).
2. Enter a short **description**.
3. Enter the **author** name.

Pairee then clones the boilerplate files from the built-in `plugin-template` branch:

```
my-plugin.pairee/
├── manifest.toml     ← name, description, author pre-filled
├── main.lua          ← ready-to-run Lua entry point
├── lang/en.toml      ← default English translation keys
├── help/en.md        ← user-facing help documentation
├── icon.png          ← 256×256 placeholder icon
└── screenshots/
    └── screenshot1.png
```

### 7.3 The `plugin-template` Branch

The file contents above come from a **dedicated orphan git branch** (`plugin-template`) in the Pairee repository — it is never shown in any plugin list. Pairee locates the local repository automatically by walking up from the binary's path. You can also set `PAIREE_REPO_DIR=/path/to/pairee` as an environment variable to override this.

If the repository is not available (e.g. installed as a standalone binary), files are generated from built-in defaults as a fallback.

### 7.4 Developer Tools Commands

| Action | Description |
|--------|-------------|
| **Init** | Create a new plugin from the template |
| **Lint** | Check all dev plugins for manifest validity and unsafe Lua calls |
| **Package** | Scan files, generate SHA-256 hashes, and output registry entry |
| **Submit** | Validate, fork the Pairee repo, and prepare a Pull Request |

> **Tip:** Edit the template for future plugins by checking out the `plugin-template` branch, modifying files, and committing. New plugins created after that point will use your updated boilerplate.

---

## 📖 8. Advanced Integration Manuals

For complex modules, please consult their dedicated documentation guides:
* **SSH & SFTP Connections:** See [SSH & SFTP Remote Connections Manual](ssh_sftp.md).
* **Git Integration:** See [Git Integration Reference Manual](git_integration.md).
* **Detailed Configurations:** See [Configuration Settings Manual](configuration_details.md).
* **Keyboard Shortcuts Cheatsheet:** See [Keyboard Shortcuts Guide](keyboard_shortcuts.md).
* **Plugin Developer Guide:** See [Plugin Developer Guide](../../docs/plugin-dev-guide.md) and [Lua API v1](../../docs/api/lua/v1.md).

