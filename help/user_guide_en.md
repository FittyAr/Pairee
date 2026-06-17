# Pairee User Guide & Configuration Manual

This guide covers installation, customization settings, themes formatting, keyboard shortcuts, and file associations for **Pairee**.

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
* **Windows:** `target/release/pairee.exe`
* **Linux/macOS:** `target/release/pairee`

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
| `F10` | Quit Pairee. |
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
* **Use system copy routine:** Delegates file copy operations to the native OS routines instead of using Pairee's internal worker streams.
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
* **Show bottom F-keys bar:** Toggle bottom F-keys bar display.
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

Themes are loaded from `%APPDATA%/pairee/config/themes/` (Windows) or `~/.config/pairee/themes/` (Linux/macOS) in TOML formats.

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

---

## 🌐 5. Using Pairee over SSH & Modifier Keys (Ctrl / Alt)

When using **Pairee** remotely over SSH, you may notice that holding down `Ctrl` or `Alt` does not automatically update the bottom F-keys bar. This is a fundamental limitation of standard terminals and the SSH protocol, which only transmit bytes when a complete key combination is pressed (they do not send events when modifier keys are pressed or released alone).

To resolve this limitation, we have implemented several options to ensure you can still easily inspect and run these key combinations:

### 1. Manual Modifier Cycling (No Third-Party Apps Required)
Inside **Pairee**, you can press **`Ctrl+p`** (or `Ctrl+P`) to cycle the bottom F-key bar states manually:
* **First Press**: Locks the bottom bar to show `CONTROL` functions (e.g. F3: Name, F4: Extension).
* **Second Press**: Locks the bottom bar to show `ALT` functions (e.g. F3: View, F4: Edit).
* **Third Press**: Restores the default F-key layout.

*Note: All shortcuts remain fully functional even if the bar is not visually showing them! For example, pressing `Ctrl+F3` sorts by name, and `Alt+F1` opens the left drive menu instantly.*

### 2. Live Modifier Tracking (via X11 Forwarding)
If you want the bottom bar to update dynamically when you hold down the physical `Ctrl` or `Alt` keys on your keyboard, you can enable **X11 Forwarding** on your SSH connection. **Pairee** will query your local X server to check the physical modifier key states.

Here are the recommended third-party client configurations to enable this:

#### 💻 Windows Host
* **MobaXterm (Recommended & Easiest)**:
  MobaXterm includes an integrated X server out-of-the-box. Simply create an SSH session, and X11 forwarding is configured automatically.
* **Windows Terminal / PowerShell / CMD (with VcXsrv)**:
  1. Download and install **VcXsrv** (or **Xming**).
  2. Launch **XLaunch** (VcXsrv) with:
     * *Multiple windows*
     * Display number: `0`
     * **Crucial**: Check **"Disable access control"** to allow connection permissions from your container/remote host.
  3. Connect using standard Windows OpenSSH client:
     ```cmd
     ssh -Y user@hostname -p port
     ```
* **PuTTY**:
  1. Expand **Connection** -> **SSH** -> **X11** in the settings tree.
  2. Check **"Enable X11 forwarding"**.
  3. Set **X display location** to `localhost:0`.
  4. Ensure you have an X Server like VcXsrv running in the background before connecting.

#### 🍎 macOS Host
* **XQuartz**:
  1. Download and install **XQuartz**.
  2. Open XQuartz, go to *Preferences* -> *Security*, and check **"Allow connections from clients"**.
  3. Connect using terminal:
     ```bash
     ssh -Y user@hostname -p port
     ```

#### 🐧 Linux Host
* Linux systems have native X11 support. Simply run:
  ```bash
  ssh -Y user@hostname -p port
  ```
