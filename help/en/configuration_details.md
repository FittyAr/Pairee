# Configuration Settings Manual

This manual provides an exhaustive, field-by-field description of all interactive options available in Pairee's Setup Dialog (`F2 -> Options -> Configuration` or `Commands -> Configuration`).

> **Note:** only options that have an effect are offered. Options from older releases that never did anything (for example file descriptions, info panel name formats, plugin manager compatibility flags, editor code pages or the external editor command) were removed; if your `config.toml` still contains them they are ignored.

---

## 📂 Tab 0: System Settings

This tab controls file processing, history recording, escalation permissions, and sorting collations.

### File Operations
* **Delete to Recycle Bin:**
  - *Description:* When enabled, deleted files are moved to the OS recycle bin (trash). If disabled, files are deleted permanently (unrecoverable without forensics).

### Optional features
* **SSH / SFTP connections:**
  - *Description:* Shows Connect SSH in the panel menus and allows the connect action. Disconnect still works if a session is already open.
* **Lua plugin system:**
  - *Description:* Loads plugins at startup and shows Plugin commands. Turning this off skips plugin load until the next launch after you re-enable it.
* **Image preview (F3 / quick view):**
  - *Description:* Decodes PNG/JPEG/etc. in the internal viewer and quick view. Off = treat images as text/hex.

### History Preservation
* **Save commands history:**
  - *Description:* Saves terminal command-line prompt history across sessions.
* **Save folders history:**
  - *Description:* Remembers recently visited paths in both panels.
* **Save view and edit history:**
  - *Description:* Stores the paths of recently viewed or edited files in history lists.

### Environment & Registry
* **Use Windows registered types:**
  - *Description:* (Windows only) Queries the system registry to load associations and descriptions.
* **Automatic update env variables:**
  - *Description:* Updates terminal environment variables (like PATH) dynamically when modifications are made in the system.

### Permissions & Elevation
* **Request admin modification:**
  - *Description:* Automatically prompts for root/administrator privilege elevation (sudo/UAC) when writing or renaming system files.
* **Request admin reading:**
  - *Description:* Prompts for privilege elevation when attempting to open or read files without access permissions.

### Sorting Collation & Saving
* **Sorting collation:**
  - *Options:* `< linguistic >` (alphabetical order) or `< natural >` (like linguistic, but numbers inside names are compared numerically, the same as *Treat digits as numbers*).
* **Treat digits as numbers:**
  - *Description:* Sorts numerically (natural sort). E.g., `file2` comes before `file10`.
* **Case sensitive sort:**
  - *Description:* Sorts uppercase names separately from lowercase names.
* **Auto save setup:**
  - *Description:* Saves all modified configuration parameters automatically to the config file when exiting Pairee.

---

## 📂 Tab 1: Panel Settings

Controls layout columns, directory display filters and updates.

### Panel Display & Selection
* **Show hidden and system files:**
  - *Description:* Toggles rendering dotfiles (Linux/macOS) and system hidden files.
* **Highlight files:**
  - *Description:* Renders files in different colors based on their file extensions.
* **Select folders:**
  - *Description:* When tagging groups (`+` or `-`), folder paths will match and be selected alongside files.

### Sorting
* **Sort folder names by extension:**
  - *Description:* Sorts directories by their folder extension suffix instead of treating folders as having no extension.
* **Sort reverse:**
  - *Description:* Reverses the sort order of the file list.
* **Show sort mode letter:**
  - *Description:* Displays a single letter indicator (e.g. `n` for Name, `s` for Size) in the status bar.

### Updates & Information
* **Disable panel update object count:**
  - *Description:* Throttles updates of item counts on extremely large folders to keep performance smooth.
* **Show files total information:**
  - *Description:* Renders aggregated counts and total bytes at the bottom status line.
* **Show free size:**
  - *Description:* Displays remaining free space on the current drive at the top panel header.

### Appearance
* **Show column titles:**
  - *Description:* Renders the headers (Name, Size, Date) above panel lists.
* **Show status line:**
  - *Description:* Shows the active selection count details.
* **Show scrollbar:**
  - *Description:* Displays vertical scrollbars in panels.
* **Show ".." in root folders:**
  - *Description:* Renders parent folder links (`..`) even when in root directories (e.g. `/` or `C:\`).

---

## 📂 Tab 2: Interface Settings

Configures UI general appearance, terminal rendering, and modal workflow.

### General
* **Clock:** Displays a live digital clock widget in the top right.
* **Mouse support:** Toggles mouse navigation, clicking, and scrolling.
* **Show bottom F-keys bar:** Toggles the F1-F10 shortcuts line at the bottom.
* **Always show the menu bar:** Toggles persistent visibility of the top menu bar.

### Keybindings
* **Keybindings preset:** Cycle Norton / Neovim / VS Code. A status line shows whether the selected preset (plus your `custom_bindings`) loaded cleanly.
* **View keymap issues:** Opens the full list of rejected chords, duplicates, and warnings. Far-style `Gray+` / `Gray-` / `Gray*` aliases map to `Plus` / `-` / `*`.

### Workflow
* **Enable Yazi workflow:**
  - *Description:* Enables Yazi/Ranger-style modal keyboard workflow. Pressing `s` opens the Sort panel and `v` opens the View panel at the bottom (only active when the command line is empty).

---

## 📂 Tab 3: Confirmations Settings

Specifies which operations require an explicit warning dialog before proceeding.

### File Operations
* **Confirm copy / move:** Prompts before performing copies or moves. What happens when a destination file already exists is decided by the transfer itself (see `transfer_conflict_resolution` in `config.toml`).
* **Confirm delete / delete non-empty folders:** Prompts before deleting items or directories containing files.
* **Confirm interrupt operation:** Ask before terminating background processes.

### General
* **Confirm reload edited file:** Asks before `Ctrl+R` in the editor reloads the file from disk and discards unsaved changes.
* **Confirm clear history list:** Prompts before wiping database lists.
* **Confirm exit:** Prompts before quitting Pairee.

---

## 📂 Tab 4: Language & Plugins Settings

### Language
* **Main language:** Selects the active translations database (detects TOML files in the `/lang` directory).

### Plugins
* **Plugins developer mode:** Enables the plugin developer tools (development folder and test plugin).

---

## 📂 Tab 5: Editor/Viewer Settings

### Viewer
* **Use external viewer for F3:** F3 runs the view command of the file's association (see *File Associations Editor* below) and `Alt+F3` opens the internal viewer; when off it is the other way round.
* **Use external command when opening files with Enter:** Enter runs the association's open command instead of opening the internal viewer.
* **Tab size / Show scrollbar:** Tab width and scrollbar of the internal viewer.
* **Detect encoding automatically:** Detects the encoding of viewed files (byte-order mark, UTF-16, UTF-8, legacy code pages). When off, **Default encoding** is used for every file (a matching byte-order mark is still skipped). `F8` in the viewer always lets you switch.
* **Default encoding:** Encoding used when detection is off (`viewer_default_codepage`, an encoding name such as `windows-1252`; Enter cycles through the list).

### Built-in Editor
Pairee edits files only with its built-in editor (`F4`); there is no external editor option.
* **Tab size:** Width of a tab stop (2, 4 or 8 columns).
* **Expand tabs:** *Do not expand tabs* inserts a tab character; *Expand new tabs to spaces* makes `Tab` insert spaces up to the next stop; *Convert all tabs to spaces* also converts the tabs already in a file when it is opened.
* **Auto indent:** `Enter` starts the new line with the indentation of the current one.
* **Show line numbers:** Shows the line-number gutter.
* **Cursor at the end:** Opens files with the cursor on the last line.
* **Lock editing of read-only files:** Read-only files open locked; use `Shift+F2` to save a copy.
* **Warn when opening read-only files:** Shows a notice when a read-only file is opened.

---

## 📂 Tab 6: Colors Settings

### Theme Configuration
* **Theme:** Apply color profiles (Slate, Blue, High Contrast).
* **Color groups / Highlighting:** Edit specific color values for UI components and customized file extensions.

---

## 📂 Tab 7: Git Settings

### General
* **Enable Git integration:** Globally enable/disable Git dashboard hooks.

### Author Identity
* **Author name / Author email:** Override author details for commits. If left blank, Pairee reads from system git config files.
* **Max log entries:** Limits how many commits to show in log listings.

---

## 🔗 File Associations Editor

File Associations allow you to map file name patterns (glob masks) to custom launch commands. This editor is accessible via **Top Menu Bar (F9) → Commands → File Associations**.

### Keyboard Shortcuts in the Editor
* `Up` / `Down`: Navigate through the list of rules.
* `A` / `a` / `Insert`: Add a new association rule. You will be prompted sequentially to enter:
  1. **Mask:** Glob pattern (e.g. `*.rs` or `*.{jpg,png}`).
  2. **Open Command:** The terminal command to run when opening the file (e.g. `explorer %f` on Windows or `xdg-open %f` on Linux). The `%f` placeholder will be replaced with the selected file path.
  3. **View Command (optional):** Command for the F3 viewer. If left blank, it falls back to the open command.
* `E` / `e` / `Enter`: Edit the highlighted rule. Follows the same step-by-step input fields as adding a rule.
* `D` / `d` / `Delete`: Delete the highlighted rule from the list.
* `Esc`: Exit the editor or cancel editing.

All changes are automatically saved to your `associations.toml` file.

---

## ⚙️ Configuration File Settings (settings.toml)

Some advanced parameters can be configured directly inside your `settings.toml` file (located in your configuration folder):

### Auto-Update Configurations
* **`auto_update_check`** (`bool`, default: `true`):
  - *Description:* When enabled, Pairee queries GitHub Releases asynchronously at launch to check for new updates.
* **`dismissed_update_version`** (`string`, default: `null` / empty):
  - *Description:* Stores the version string (e.g., `v1.2.3`) of an update that was explicitly ignored or dismissed by the user, preventing future notification popups for that specific release. You can clear this value to re-enable checks for that version.

