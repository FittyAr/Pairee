# NCRust User Guide & Configuration Manual

This guide covers installation, customization settings, themes formatting, keyboard shortcuts, and file associations for **NCRust**.

---

## 🛠️ 1. Build & Installation Guide

### Compilation
Build the binary from the source directory using Cargo:
```bash
# Debug mode build (includes symbols)
cargo build

# Release mode build (optimized, stripped of debug logs)
cargo build --release
```
The compiled output is located at:
* **Windows:** `target/release/ncrust.exe`
* **Linux/macOS:** `target/release/ncrust`

---

## ⌨️ 2. Comprehensive Keyboard Shortcuts Cheatsheet

### 2.1 General & Navigation
| Key / Shortcut | Action |
| :--- | :--- |
| `Tab` | Switch focus between the active and passive panels. |
| `Up / Down` | Move cursor focus. |
| `PageUp / PageDown` | Scroll the active file panel list up or down by one screen. |
| `Home / End` | Jump to the very beginning or end of the list. |
| `Ctrl+U` | Swap directory paths between the Left and Right panels. |
| `Ctrl+H` | Toggle hidden files/folders. |
| `Ctrl+R` | Reread/refresh the current panel directory. |
| `Ctrl+\` | Open the Directory Bookmarks Hotlist. |
| `Alt+F8` | Open the Command Line history dialog. |
| `Alt+F12` | Open the Folder navigation history dialog. |
| `Ctrl+PgUp` / `Ctrl+PgDn` | Swap/select active drives on the panel path. |
| `Alt+F1` / `Alt+F2` | Open Drive Select list for Left / Right panels. |

### 2.2 Screen & Tab Management
| Key / Shortcut | Action |
| :--- | :--- |
| `Ctrl+Tab` / `Ctrl+Right` | Switch focus to the next open Screen tab context. |
| `Ctrl+Shift+Tab` / `Ctrl+Left` | Switch focus to the previous open Screen tab context. |
| `F2 -> Commands -> Screens` | Open the Screens Menu overlay to see active tabs list. |

### 2.3 Panel Visibility & Toggles
| Key / Shortcut | Action |
| :--- | :--- |
| `Ctrl+F1` | Show / hide the Left Panel. |
| `Ctrl+F2` | Show / hide the Right Panel. |
| `Ctrl+O` | Toggle visibility of both panels. |

### 2.4 File Actions
| Key | Action |
| :--- | :--- |
| `F1` | Open in-app Help and keyboard action overlay. |
| `F2` | Open dropdown actions menu on the top bar. |
| `F3` | Open internal file viewer (text/hex modes). |
| `F4` | Open internal file editor. |
| `F5` | Copy highlighted/selected files to passive panel destination. |
| `F6` | Rename or Move highlighted/selected files to passive panel destination. |
| `F7` | Create a new directory (MkDir). |
| `F8` | Delete highlighted/selected files. |
| `F9` | Activate Top Menu bar options. |
| `F10` | Quit NCRust. |
| `Esc` | Close popups/dropdowns or clear Command Line buffers. |
| `Shift+F10` | Open the Context actions menu. |
| `Ctrl+L` / `Alt+F6` | Open create Hardlink / Symlink prompt dialog. |
| `Ctrl+D` | Edit file description tags (`Descript.ion`). |

### 2.5 Selection Modes
| Key | Action |
| :--- | :--- |
| `Insert` / `Space` | Toggle tagged selection state on the currently focused file. |
| `+` (Keypad) | Select all files matching a wildcard mask (e.g. `*.txt`). |
| `-` (Keypad) | Deselect files matching a wildcard mask. |
| `*` (Keypad) | Invert tagged selection for the entire panel list. |

---

## ⚙️ 3. Settings Config Dialog (`F2 -> Options -> Configuration`)

The Setup dialog is divided into tabbed categories:

### Tab 0: System Settings
* **Delete to Recycle Bin:** Toggle sending deleted files to system trash bin.
* **Use system copy routine:** Delegates file copy operations to the native OS routines instead of using NCRust's internal worker streams.
* **Copy files opened for writing:** Toggles ability to copy files currently locked by other applications.
* **Sorting collation:** Sets collation mode to `linguistic` (natural sort order) or `binary` (direct byte comparisons).
* **Treat digits as numbers:** If active, `file2` is sorted before `file10`.
* **Case sensitive sort:** Order capitalized files separately from lowercase files.
* **Scan symbolic links:** Traverses symbolic link paths when parsing files.
* **Save commands history:** Saves terminal console command entries to persistent database files.
* **Save folders history:** Saves visited paths history.
* **Save view and edit history:** Remembers recently opened editor/viewer files.
* **Auto save setup:** Saves options automatically on exit.

### Tab 1: Panel Settings
* **Show hidden and system files:** Show dotfiles and OS-hidden files.
* **Highlight files:** Renders extension-specific colors.
* **Select folders:** Folder paths match against selection wildcard masks.
* **Right click selects files:** Hides/shows right-click action toggles.
* **Sort folder names by extension:** Apply active sort field to directory extensions.
* **Show column titles:** Toggles display of headers at the top of the panels.
* **Show status line:** Shows tagged file statistics.
* **Show scrollbar:** Displays scrollbars for panels.
* **Show ".." in root folders:** Display parent folder navigation links when in the root directories.

### Tab 2: Interface Settings
* **Clock:** Display clock widget in the top right.
* **Show key bar:** Toggle bottom F-keys bar display.
* **Always show the menu bar:** Keep top menu visible.
* **Show total copy progress indicator:** Displays progress bars for file copy jobs.
* **Show total delete progress indicator:** Displays progress bars for delete operations.
* **Keybindings preset:** Switch keyboard preset profile: `"norton"`, `"vim"`, or `"modern"`.

### Tab 4: Language & Plugins Settings
* **Main language:** Change localization database (e.g., `"English"` or `"Spanish"`).
* **OEM plugins support:** Load compatibility plugins.

### Tab 5: Editor/Viewer Settings
* **Use external editor for F4:** Redirect edit actions to an external command.
* **Editor command:** Target execution string for external edits (e.g., `nano %f`).
* **Use external viewer for F3:** Redirect view actions to an external command.
* **Viewer command:** Target execution string for external viewing (e.g., `less %f`).
* **Tab size:** Set spaces count mapping to tab hits.
* **Show line numbers:** Displays line indexes in the editor.

### Tab 6: Colors Settings
* **Theme:** Apply styling theme profiles (Slate, Blue, High Contrast).

---

## 🎨 4. Custom TOML Themes

Themes are loaded from `%APPDATA%/ncrust/config/themes/` (Windows) or `~/.config/ncrust/themes/` (Linux/macOS) in TOML formats.

### Theme Properties Map
```toml
[panel]
border = "Blue"              # Panel border frame color
background = "Black"          # Panel inner background
file_selected = "Yellow"      # Color of tagged items
file_directory = "Cyan"       # Color of folder items
file_executable = "Green"     # Color of binaries/scripts

[menu]
background = "Blue"          # Top menu background
selected = "White"            # Selected item text color
```
Supported colors: `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `Gray`, `DarkGray`, `Reset`, or custom hexadecimal numbers (`#RRGGBB`).
