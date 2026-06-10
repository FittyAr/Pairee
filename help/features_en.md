# NCRust Features Reference Manual

This manual provides an exhaustive description of every interactive feature, utility, and console integration available in **NCRust**.

---

## 🖥️ 1. Panel Views & Custom Layouts

NCRust utilizes a dual-panel architecture for folder navigation and file management.

### 1.1 Panel Display Modes
You can configure each panel independently to display files using different columns and detail levels:
* **Brief:** Displays file names only across multiple columns. Ideal for navigating directories with thousands of files.
* **Medium:** Lists file name and file extension side-by-side.
* **Full / Detailed:** Displays comprehensive filesystem metadata:
  * **Name & Extension:** Complete filename and extension.
  * **Size:** File size in bytes (directories display `<DIR>`).
  * **Date Modified:** Last modified timestamp.
  * **Permissions:** Unix octal notations (e.g. `0755`) or attributes.
  * **Owner:** The username or user group of the file owner.
  * **Links:** The count of hardlinks pointing to this physical file.
* **Wide:** Broad name listing with minimal details.
* **Descriptions:** Renders file name along with description details loaded from `Descript.ion` lists.
* **FileOwners:** Lists files alongside user/group names.
* **FileLinks:** Lists files with hardlink count columns.
* **AltFull:** User-configurable custom column structure.

### 1.2 Panel Visibility & Swapping
* **Toggle Left/Right Panel (`Ctrl+F1` / `Ctrl+F2`):** Individually show or hide the left or right panels.
* **Toggle Both Panels (`Ctrl+O`):** Hides both panels to inspect the terminal outputs of previous background executions.
* **Swap Panels (`Ctrl+U`):** Instantly swap the paths of the left and right panels.
* **Navigation History (`Alt+F12`):** Displays a popup listing recently visited directories. Select a row and press `Enter` to jump directly to that directory.
* **Directory Hotlist (`Ctrl+\`):** A custom bookmarks list for adding, deleting, and selecting your most visited folders.

---

## 📂 2. File System Operations

File operations are fully asynchronous and process on a background worker queue (`tokio`).

### 2.1 Bulk Selection & Tagging
* Tag files by pressing `Insert` or `Space` on an item. The cursor automatically moves down.
* Use `+` (Keypad) to tag files using wildcard patterns (e.g. `*.rs` or `temp_*`).
* Use `-` (Keypad) to untag files using wildcard patterns.
* Use `*` (Keypad) to invert the selection state of the entire panel.
* **File Panel Filter (`Alt+F9` / Options):** Apply an active glob filter (e.g., `*.rs`) to restrict visible items in the current panel list.

### 2.2 Copy & Move/Rename (`F5` & `F6`)
* **Background Processing:** Both copy and move tasks run asynchronously, showing real-time progress bars, byte transfer counts, file names, and percentages.
* **Overwrite Resolution:** If a file exists at the target path, NCRust prompts with:
  * *Ask:* Prompts the user with an overwrite warning dialog.
  * *Overwrite:* Overwrites the existing destination file.
  * *Skip:* Skips copying/moving that specific file and continues with the rest.
  * *Append:* Appends the contents of the source file to the destination file.
* **Symbolic Links Options:**
  * *Smartly copy:* Copies the symlink pointer if the destination supports it; otherwise, copies the physical target data.
  * *Copy link:* Copies the symlink pointer itself.
  * *Copy target:* Resolves the symlink and copies the target data.

### 2.3 Secure Wipe & Deletion
* **Normal Delete (`F8`):** Moves files/folders to the system recycle bin or deletes them permanently depending on your settings.
* **Secure Wipe:** Overwrites file sectors with random byte buffers before removal, rendering the data completely unrecoverable by forensic tools.

### 2.4 Creating Links (`Ctrl+L` / Alt+F6)
* Easily create symbolic links or hard links mapping a source file or directory to a specific destination path.

---

## 🔍 3. Search, Viewer, & Editor

### 3.1 Advanced Search (`Alt+F7`)
* **Filters:** Search folders recursively using wildcard name masks (e.g., `*.toml`, `src*`).
* **Content Search:** Search for specific text strings inside files.
* **Result Navigation:** The search results popup lists all matches. Select any match and press `Enter` to close the search and jump directly to that file in the active panel.

### 3.2 Internal Viewer (`F3`) & Quick View (`Ctrl+Q`)
* **Viewer Modes:** Toggle between plain Text mode and Hex Dump mode.
* **Hex Dump View:** Displays offsets, hex values, and ASCII representation side-by-side. Excellent for inspecting binary files.
* **Viewer Search:** Press `F7` inside the viewer to search for text strings.
* **Quick View (`Ctrl+Q`):** Instantly displays a preview of the highlighted file in the opposite panel.
  * Preview text files directly.
  * Lists archive metadata, compression ratio, and file counts for archive files (ZIP, TAR).
  * Displays a `[Binary file — cannot preview]` warning for non-text formats.

### 3.3 Internal Editor (`F4`)
* Edit text files directly in-app.
* Features status counters (current line, character count) and dirty state alerts when attempting to exit without saving.

---

## 🛠️ 4. In-App Screen Management (Multitasking)

NCRust features a robust multitasking screens architecture. You can spawn several work environments concurrently (e.g., editing one file, viewing another, running a terminal execution, and browsing file panels).

### 4.1 Screens List Overlay (`F2 -> Commands -> Screens list`)
* View a list of all active open screens. The active screen is marked with an asterisk (`*`).
* Select any screen and press `Enter` to switch to it instantly.
* **Suspend/Resume Popups:** Switching screens preserves the state of active popup dialogues. For example, if you are midway through a copy prompt dialog, you can open the Screens Menu, check another file in the Editor, and switch back to resume the copy prompt dialog exactly where you left off.

### 4.2 Next / Previous Screen Actions
* Use hotkeys to cycle forward or backward through your open screen contexts without opening the menu.

---

## 🧰 5. Utilities & Advanced Tools

### 5.1 Context Actions Menu
* Opens a popup menu containing actions (View, Edit, Copy, Move, Delete, Compress, Extract) relative to the highlighted file type.
* Detects archives (ZIP, 7z, RAR, TAR, GZ, BZ2, XZ) and adds dynamic Archive Commands Option lists (e.g., Extract) to the menu automatically.

### 5.2 Folder Compare
* Compares left and right panel directory listings.
* Identifies files that are present in only one panel or differ in size or modification date.
* Automatically highlights and tags mismatched files so you can sync them with a single copy/move action.

### 5.3 OS Task Manager (`Alt+F9`)
* Displays a table of active system processes with their PIDs, names, and memory consumption.
* Terminate selected processes directly from the UI using `Delete` or `Alt+Delete`.

### 5.4 Directory Tree View (`Alt+F10` / Tree Button)
* Traverses the directory structure and displays a graph-like tree layout.
* Navigate the tree structure and press `Enter` to jump the active panel directly to the selected directory. Also accessible within Copy and Move dialogs to graphically choose destination paths.

### 5.5 File Descriptions (`Ctrl+D`)
* NCRust supports `Descript.ion` and `Files.bbs` files to store descriptions for files.
* View and edit descriptions using `Ctrl+D` on a file. NCRust automatically saves changes to hidden description files in the directory.

### 5.6 File Associations
* Map file extensions (e.g., `*.py`, `*.rs`) to custom launch commands.
* Configure custom actions for opening (`Enter`) or viewing (`F3`) specific file types.

### 5.7 Custom User Commands Menu (`F2 -> Commands -> Edit user menu`)
* Define custom shell commands or script execution shortcuts to run on highlighted or tagged files.

### 5.8 Drive Select Panel (`Alt+F1` / `Alt+F2` / `Ctrl+PgUp`)
* Displays removable disks, external USB drives, and mounted network drives, allowing you to switch panel paths to them instantly.

### 5.9 System Info Panel (`F2 -> Commands -> Info panel`)
* Overlay window displaying current OS distribution name, machine NetBIOS hostname, logged-in username, available system RAM, and environment parameters.
