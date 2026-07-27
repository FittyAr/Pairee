# 🎓 Tutorial: Getting Started with Pairee

> **Quadrant: TUTORIAL** — *learning-oriented, hands-on, no prior knowledge assumed.*

In the next ten minutes you will install Pairee, launch it, and complete a
small real task (copy a folder tree, safely). Every step is concrete; if
you get stuck, jump to the reference doc linked at the bottom.

---

## 1. Install Pairee

### One-liner (recommended)

**Linux / macOS** (run in any shell):

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
```

**Windows** (run in PowerShell):

```powershell
irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
```

The script detects your platform, downloads the matching release from
GitHub, verifies the SHA-256, and places the binary in a sensible
location (`/usr/local/bin` on Linux/macOS, `%LOCALAPPDATA%\Programs\pairee`
on Windows).

### From source

If you prefer to compile (or want the bleeding edge):

```bash
git clone https://github.com/FittyAr/Pairee.git
cd Pairee
cargo build --release
./target/release/pairee         # or .\target\release\pairee.exe on Windows
```

You need **Rust 1.70+** installed. Build artifacts and dev dependencies
take ~3 min on a modern machine.

> Need to uninstall? See [`29_howto_install_build_update`](29_howto_install_build_update.md).

---

## 2. Launch Pairee

Open a terminal and type:

```bash
pairee
```

You should see a two-panel TUI, with the **left** and **right** panels
each showing a directory listing, an **F-key bar** at the bottom
(`1 Help  2 User  3 View  …`), a top **menu bar** (`Left | Files |
Commands | Options | Right | Help`), and a **clock** in the top-right
corner.

> If the terminal looks glitchy, see the troubleshooting note at the end.

---

## 3. The 60-second tour

Try each of the following in order. None of these steps modify your disk.

| Step | Action | What to notice |
| --- | --- | --- |
| 1 | Press `Tab` | Focus jumps to the other panel. |
| 2 | Press `Up` / `Down` (or `j` / `k`) | The highlight moves; the other panel is untouched. |
| 3 | Press `F1` | The help popup opens with this documentation. `Esc` closes it. |
| 4 | Press `Ctrl+1`, `Ctrl+2`, `Ctrl+3` | The active panel switches between *Brief*, *Medium*, and *Full* views. |
| 5 | Press `F2` | A small **User Menu** pops up with quick commands. |
| 6 | Press `F9` | The top menu drops down. Use arrows + `Enter` to navigate, or press the highlighted letter (`&`-accelerator). |
| 7 | Press `Ctrl+O` | Both panels collapse, exposing the raw screen. Press again to bring them back. |
| 8 | Press `F12` | The **Screens** overlay lists every screen you have open. Press `Esc` to close. |

You now know enough to drive Pairee without reading further. The next
sections do a single realistic task.

---

## 4. Your first real task: copy a folder safely

Goal: copy a folder called `~/projects/notes` to `~/backup/`.

1. **Navigate the left panel to the source**:
   - Type a path: press `Shift+Tab` to focus the command line, type the
     absolute path, press `Enter`.
   - Or click around: `Tab` to switch panels, arrow keys to move,
     `Enter` to enter a folder, `Backspace` to go up one level.

2. **Navigate the right panel to the destination**:
   - Press `Tab` to focus the right panel, then repeat.

3. **Tag the source folder**:
   - Move the highlight onto `notes`.
   - Press `Insert` (or `Space`). The cursor jumps to the next item
     and the file gets a small selection marker (colour depends on
     your theme).

4. **Copy it**:
   - Press `F5` (or `Enter` on the tag and then `F5`).
   - A small dialog asks for the destination. Confirm.

5. **Watch the transfer**:
   - A progress popup shows the current file, transfer rate, and ETA.
   - You can keep navigating the panels while the copy runs in the
     background. The progress popup stays on top until done.

6. **Verify**:
   - Press `Ctrl+R` to refresh the right panel.
   - The `backup/notes` folder is there. You can press `F3` on it to
     peek inside, `F4` to edit a file, `Enter` to dive in.

7. **Undo (if needed)**:
   - There is no global undo, but you can `F8` to delete what you
     copied, and from the next dialog choose **Move to Recycle Bin**
     (default if the *Delete to Recycle Bin* option is enabled in
     `41_reference_configuration`, Tab 0).

🎉 You just used: panel focus, navigation, tagging, copy, async
progress, and refresh — the daily-driver loop.

---

## 5. Where to go next

| If you want to… | Read |
| --- | --- |
| Learn more about the two panels, view modes, and screens | [`11_tutorial_panels_and_screens`](11_tutorial_panels_and_screens.md) |
| Find a specific file or filter the panel | [`21_howto_search_filter_history`](21_howto_search_filter_history.md) |
| Move/delete/wipe/links/attributes | [`20_howto_file_operations`](20_howto_file_operations.md) |
| Pack or unpack archives | [`22_howto_archives`](22_howto_archives.md) |
| Connect to a remote server | [`23_howto_ssh_sftp`](23_howto_ssh_sftp.md) |
| Manage a Git repository | [`24_howto_git_integration`](24_howto_git_integration.md) |
| Change the look (themes, layouts, F-key bar) | [`25_howto_appearance_themes`](25_howto_appearance_themes.md) |
| Switch keymap presets (Norton/Neovim/VSCode) | [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md) |
| Install or write plugins | [`28_howto_plugins`](28_howto_plugins.md) |

---

## Troubleshooting

| Symptom | Likely fix |
| --- | --- |
| Terminal looks glitchy (lines repaint wrong) | Try a modern terminal: **Windows Terminal**, **WezTerm**, **Alacritty**, **kitty**. Disable ClearType via `Configuration → Interface → ClearType friendly redraw` if you are on Windows Console host. |
| `pairee: command not found` | The install script did not put the binary on `PATH`. Re-run it, or add `~/.local/bin` (Linux/macOS) or `%USERPROFILE%\.cargo\bin` (Windows) to your `PATH`. |
| The F-key bar is wrong over SSH | `Ctrl` / `Alt` modifiers don't travel through SSH. See [`30_howto_ssh_modifier_keys`](30_howto_ssh_modifier_keys.md). |
| I want my old F1-F10 / F2 menu back | The keymap might be `neovim` or `vscode`. See [`26_howto_keymaps_user_menu`](26_howto_keymaps_user_menu.md). |
| The screen flashes or scrolls on each redraw | `ratatui` requires an alternate-screen terminal. All terminals listed above support it. If you are inside `tmux`/`screen`, enable the option `terminal-override` and use `-2`. |
