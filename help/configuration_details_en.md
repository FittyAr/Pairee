# Configuration Settings Manual

This manual provides an exhaustive, field-by-field description of all interactive options available in Pairee's Setup Dialog (`F2 -> Options -> Configuration` or `Commands -> Configuration`).

---

## 📂 Tab 0: System Settings

This tab controls file processing, history recording, escalation permissions, and sorting collations.

### File Operations
* **Delete to Recycle Bin:**
  - *Description:* When enabled, deleted files are moved to the OS recycle bin (trash). If disabled, files are deleted permanently (unrecoverable without forensics).
* **Use system copy routine:**
  - *Description:* Delegates copy/move operations to the native system APIs. Disabling this uses Pairee's optimized internal Tokio worker thread streams, which support custom overwrite/skip modes.
* **Copy files opened for writing:**
  - *Description:* Toggles whether Pairee attempts to copy files that are currently locked or modified by other applications.
* **Scan symbolic links:**
  - *Description:* Traverses and follows symbolic links (symlinks) during directory operations.

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
* **Request admin use additional privileges:**
  - *Description:* Allows using system privilege escalation helpers for advanced actions.

### Sorting Collation & Saving
* **Sorting collation:**
  - *Options:* `< linguistic >` (Natural human sorting, e.g. `a` then `B` then `c`) or `< binary >` (ASCII byte comparison sorting, e.g. `B` before `a`).
* **Treat digits as numbers:**
  - *Description:* Sorts numerically (natural sort). E.g., `file2` comes before `file10`.
* **Case sensitive sort:**
  - *Description:* Sorts uppercase names separately from lowercase names.
* **Auto save setup:**
  - *Description:* Saves all modified configuration parameters automatically to the config file when exiting Pairee.

---

## 📂 Tab 1: Panel Settings

Controls layout columns, directory display filters, updates, and file descriptions.

### Panel Display & Selection
* **Show hidden and system files:**
  - *Description:* Toggles rendering dotfiles (Linux/macOS) and system hidden files.
* **Highlight files:**
  - *Description:* Renders files in different colors based on their file extensions.
* **Select folders:**
  - *Description:* When tagging groups (`+` or `-`), folder paths will match and be selected alongside files.
* **Right click selects files:**
  - *Description:* Enables right-clicking to mark/tag files instead of triggering context menus.

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
* **Network drives autorefresh:**
  - *Description:* Dynamically watches and updates file lists on mounted network paths.
* **Detect volume mount points:**
  - *Description:* Scans volume tables to resolve mounting changes.
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
* **Show background screens number:**
  - *Description:* Renders a count of open screens in the background.
* **Show ".." in root folders:**
  - *Description:* Renders parent folder links (`..`) even when in root directories (e.g. `/` or `C:\`).

### Info Panel & Descriptions
* **Computer/User name formats:**
  - *Description:* Configure how hostname and username are rendered in the overlay Info panel.
* **File Descriptions:**
  - *Description:* Set lists names (e.g. `Descript.ion`), hidden flags, ANSI color support, UTF-8 formats, and updates mode for file descriptions.

---

## 📂 Tab 2: Interface Settings

Configures UI general appearance, terminal rendering, and modal workflow.

### General
* **Clock:** Displays a live digital clock widget in the top right.
* **Mouse support:** Toggles mouse navigation, clicking, and scrolling.
* **Show bottom F-keys bar:** Toggles the F1-F10 shortcuts line at the bottom.
* **Always show the menu bar:** Toggles persistent visibility of the top menu bar.
* **Screen saver minutes:** Triggers a terminal screensaver after idle time.

### Progress Indicators
* **Show total copy progress / copying time:** Shows aggregated progress bar and elapsed/estimated time during bulk copying.
* **Show total delete progress:** Shows progress indicators during bulk deletion.

### Terminal & Rendering
* **Use Ctrl+PgUp to change drive:** Allows changing active drive path using `Ctrl+PgUp`/`Ctrl+PgDn`.
* **Use virtual terminal:** (Windows) Configures console virtual terminal support.
* **ClearType friendly redraw:** Tweaks redraw patterns to prevent visual font glitches.
* **Window Title Format:** Define title bar format tokens (e.g. `%Platform`).

### Workflow
* **Enable Yazi workflow:**
  - *Description:* Enables Yazi/Ranger-style modal keyboard workflow. Pressing `s` opens the Sort panel and `v` opens the View panel at the bottom (only active when the command line is empty).

---

## 📂 Tab 3: Confirmations Settings

Specifies which operations require an explicit warning dialog before proceeding.

### File Operations
* **Confirm copy / move / overwrite:** Prompts before performing copies, moves, or overwriting destination files.
* **Confirm drag and drop:** Warns before executing mouse drag and drop actions.
* **Confirm delete / delete non-empty folders:** Prompts before deleting items or directories containing files.

### Drives & System
* **Confirm interrupt operation:** Ask before terminating background processes.
* **Confirm disconnect network drive / delete subst disk:** Prompts before disconnecting mount points.
* **Confirm detach virtual disk / hotplug removal:** Prompts before detaching virtual files.

### General
* **Confirm reload edited file:** Asks to reload if the file being edited is modified externally on disk.
* **Confirm clear history list:** Prompts before wiping database lists.
* **Confirm exit:** Prompts before quitting Pairee.

---

## 📂 Tab 4: Language & Plugins Settings

### Language
* **Main language:** Selects the active translations database (detects JSON files in the `/lang` directory).

### Plugins Configuration
* **OEM plugins support / Scan symlinks:** Enables loading legacy plugins and scanning symbolic links.
* **Plugins Selection:** Configure file processing, command prefix matching, and standard file associations.

---

## 📂 Tab 5: Editor/Viewer Settings

### External Commands
* **Use external editor / Editor command:** Redirects F4 Edit actions to an external program (e.g. `nano %f`).
* **Use external viewer / Viewer command:** Redirects F3 View actions to an external program (e.g. `less %f`).

### Internal Editor
* **Tab size:** Set spaces count mapping to tab hits.
* **Expand tabs:** Toggles expanding tabs into spaces.
* **Persistent blocks / Del removes blocks:** Controls selection block behaviour.
* **Cursor beyond EOL:** Allows positioning the cursor beyond the end of a line.
* **Show line numbers / whitespace / scrollbar:** Toggles formatting elements.

---

## 📂 Tab 6: Colors Settings

### Theme Configuration
* **Theme:** Apply color profiles (Slate, Blue, High Contrast).
* **Color groups / Highlighting:** Edit specific color values for UI components and customized file extensions.

---

## 📂 Tab 7: Git Settings

### General
* **Enable Git integration:** Globally enable/disable Git dashboard hooks.
* **Auto-detect git repos:** Scans folder trees for active git repositories.

### Author Identity
* **Author name / Author email:** Override author details for commits. If left blank, Pairee reads from system git config files.
* **Max log entries:** Limits how many commits to show in log listings.
