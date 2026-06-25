# Pairee User Guide & Configuration Manual

This guide covers compilation, installation, customization presets, and custom themes formatting for **Pairee**.

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

### Installation
You can place the binary in your system path (e.g. `/usr/local/bin/` or `C:\Windows\System32\`) or run it directly from the target folder.
Make sure the `lang/` and `help/` directories are located alongside the executable or in the system share path (`/usr/share/pairee/` on Linux) to ensure localizations and manuals load correctly.

---

## 🎨 2. Custom TOML Themes

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

## 🌐 3. Using Pairee over SSH & Modifier Keys (Ctrl / Alt)

When using **Pairee** remotely over SSH, you may notice that holding down `Ctrl` or `Alt` does not automatically update the bottom F-keys bar. This is a fundamental limitation of standard terminals and the SSH protocol, which only transmit bytes when a complete key combination is pressed (they do not send events when modifier keys are pressed or released alone).

To resolve this limitation, we have implemented several options to ensure you can still easily inspect and run these key combinations:

### 3.1 Manual Modifier Cycling (No Third-Party Apps Required)
Inside **Pairee**, you can press **`Ctrl+p`** (or `Ctrl+P`) to cycle the bottom F-key bar states manually:
* **First Press**: Locks the bottom bar to show `CONTROL` functions (e.g. F3: Name, F4: Extension).
* **Second Press**: Locks the bottom bar to show `ALT` functions (e.g. F3: View, F4: Edit).
* **Third Press**: Restores the default F-key layout.

*Note: All shortcuts remain fully functional even if the bar is not visually showing them! For example, pressing `Ctrl+F3` sorts by name, and `Alt+F1` opens the left drive menu instantly.*

### 3.2 Live Modifier Tracking (via X11 Forwarding)
If you want the bottom bar to update dynamically when you hold down the physical `Ctrl` or `Alt` keys on your keyboard, you can enable **X11 Forwarding** on your SSH connection. **Pairee** will query your local X server to check the physical modifier key states.

Here are the recommended third-party client configurations to enable this:

#### 💻 Windows Host
* **MobaXterm (Recommended & Easiest)**:
  MobaXterm includes an integrated X server out-of-the-box. Simply create an SSH session, and X11 forwarding is configured automatically.
* **Windows Terminal / PowerShell / CMD (with VcXsrv)**:
  1. Download and install **VcXsrv** (or **Xming**).
  2. Launch **XLaunch** (VcXsrv) with:
     - *Multiple windows*
     - Display number: `0`
     - **Crucial**: Check **"Disable access control"** to allow connection permissions from your container/remote host.
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

---

## 📖 4. Advanced Integration Manuals

For complex modules, please consult their dedicated documentation guides:
* **SSH & SFTP Connections:** See [SSH & SFTP Remote Connections Manual](file:///home/fitty/GitHub/Pairee/help/ssh_sftp_en.md).
* **Git Integration:** See [Git Integration Reference Manual](file:///home/fitty/GitHub/Pairee/help/git_integration_en.md).
* **Detailed Configurations:** See [Configuration Settings Manual](file:///home/fitty/GitHub/Pairee/help/configuration_details_en.md).
* **Keyboard Shortcuts Cheatsheet:** See [Keyboard Shortcuts Guide](file:///home/fitty/GitHub/Pairee/help/keyboard_shortcuts_en.md).
