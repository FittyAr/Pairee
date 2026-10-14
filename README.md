# Pairee

> Tu mundo, en dos paneles. (Your world, in two panels.)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg)](https://www.rust-lang.org)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/FittyAr/Pairee)
![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/FittyAr/Pairee?utm_source=oss&utm_medium=github&utm_campaign=FittyAr%2FPairee&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

## Pairee - ⚡ Modern Dual-Panel Terminal File Manager

Pairee is a modern, highly modular, and cross-platform terminal file manager inspired by the classic **Norton Commander** and **Far Manager**. Built in Rust utilizing `ratatui` and `crossterm`, it aims to provide an efficient, fast, and extensible file management experience directly within your console.

- 🚀 **Full Asynchronous Support**: All long-running filesystem operations (Copy, Move, Wipe, Delete) run on background threads (`tokio`), keeping the UI perfectly responsive.
- 💪 **Async Task Scheduling**: Real-time progress tracking, progress bar popups, and task cancellation for concurrent workers.
- 🎨 **Visual Themes & Layouts**: Customizable theme loader supporting Slate, Blue, and other custom styles, alongside responsive layout division.
- ⌨️ **Keys you already know**: presets that keep the keys of Norton Commander / Far, Windows Explorer / VS Code / Total Commander, Neovim and yazi, a searchable keyboard-shortcuts list where any key can be changed, and plugin commands in the same keymap ([help/en/keyboard_shortcuts.md](help/en/keyboard_shortcuts.md)).
- 🔌 **Extensible Plugin System**: Sandboxed Lua plugins (`mlua`) with real confirm/input/which dialogs, `pairee.cx` + `File` userdata, `pairee.fs` extras, and `pairee.Command` streaming. Lua API **v1.3.0** ([docs/api/lua](docs/api/lua/README.md)).
- 🧰 **Advanced Utilities**: Built-in file search by name/content, folder comparison, custom user commands menu, OS process manager, and file attributes viewer.
- 📦 **Smart Auto-Updates**: Secure checks supporting auto-detection across 13 installation methods, with SHA-256 checksums and mandatory minisign signature verification.
- 🌐 **Centralized Translations**: Core translation engine supporting English, Spanish, and easily extendable to new languages.

---

## Project Status

Public beta. Stable enough to be used as a daily driver. Pairee is in active development, and suggestions or contributions are highly welcome.

---

## 📂 Project Structure

```text
Pairee/
├── Cargo.toml                     # Cargo configuration
├── install.sh / install.ps1       # Quick installers
├── docs/
│   ├── IMPROVEMENT_PLAN.md        # Improvement checklist & progress
│   ├── README.md                  # Documentation index (status legend)
│   ├── api/lua/                   # Versioned Lua plugin API (semver)
│   └── technical/                 # Architecture, transfer, plugins, packaging
├── tests/
│   └── plugin_acceptance/         # Lua plugins run by CI (`cargo test`)
├── help/
│   ├── en/                        # In-app help (English)
│   └── es/                        # In-app help (Spanish)
├── lang/                          # UI translations (en.toml, es.toml)
├── keymaps/                       # Keybinding presets
├── .agents/                       # AI developer guidelines & skills
└── src/
    ├── main.rs                    # Entry point
    ├── app/                       # Event loop, actions, state
    ├── config/                    # Settings, themes, localization
    ├── fs/                        # Filesystem + transfer engine
    ├── git/                       # Git integration
    ├── keybindings/               # Action resolver & presets
    ├── plugin/                    # Lua plugin runtime & registry
    ├── ui/                        # Ratatui rendering
    ├── terminal/                  # Backend & standalone launcher
    └── update/                    # Auto-update pipeline
```

---

## 🛠️ Quick Start

### Installation (Quick Installers)

Pairee provides precompiled binaries built automatically via GitHub Actions (Windows x64 and arm64, Linux x64 and arm64 with static MUSL builds plus a x64 GNU build, macOS x64 and arm64). You can install them instantly via command line:

* **Linux (statically linked, standalone binary, x86_64 and aarch64):**
  * **Standard Release:**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
    ```
    The installer verifies the release signature with [`minisign`](https://jedisct1.github.io/minisign/): the installed one, or else the official build, which it downloads itself (version and SHA-256 pinned in the script). Add `--insecure-skip-signature` (`... | sh -s -- --insecure-skip-signature`) to accept the download on its SHA-256 checksum alone.
  * **Build from Source (Debug/Source Mode):**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- debug
    ```
  * **Uninstall:**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- uninstall
    ```

* **Windows (PowerShell):**
  * **Standard Release:**
    ```powershell
    irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
    ```
    The installer verifies the release signature with the official [`minisign`](https://jedisct1.github.io/minisign/) build, which it downloads itself (version and SHA-256 pinned in the script). To accept the download on its SHA-256 checksum alone, pass `-InsecureSkipSignature`:
    ```powershell
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) -InsecureSkipSignature
    ```
  * **Build from Source (Debug/Source Mode):**
    ```powershell
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) -DebugMode
    ```
  * **Uninstall:**
    ```powershell
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) -Uninstall
    ```

### Build from Source

#### Prerequisites
Make sure you have [Rust](https://www.rust-lang.org/tools/install) **1.96 or higher** (edition 2024 / MSRV declared in `Cargo.toml`).

#### Build and Run
```bash
# Clone the repository
git clone https://github.com/FittyAr/Pairee.git
cd Pairee

# Run Pairee in development mode
cargo run

# Build the release binary
cargo build --release
```

### Command Line & Shell Integration
```sh
pairee [--cwd-file <file>] [--print-cwd] [--no-mouse] [--standalone] [<left folder> [<right folder>]]
```
The last session (tabs, folders, layout) is restored at startup (**Restore last session** setting); folders given on the command line win. `--cwd-file` writes the focused panel's folder on exit so a shell function can `cd` there:

```sh
# bash / zsh (~/.bashrc, ~/.zshrc)
p() {
  local tmp dir
  tmp="$(mktemp)"
  command pairee --cwd-file "$tmp" "$@"
  dir="$(cat -- "$tmp")"
  [ -n "$dir" ] && [ "$dir" != "$PWD" ] && cd -- "$dir"
  rm -f -- "$tmp"
}
```

```fish
# fish (~/.config/fish/functions/p.fish)
function p
    set tmp (mktemp)
    command pairee --cwd-file $tmp $argv
    set dir (cat -- $tmp)
    if test -n "$dir"; and test "$dir" != "$PWD"
        cd -- $dir
    end
    rm -f -- $tmp
end
```

```powershell
# PowerShell ($PROFILE)
function p {
    $tmp = New-TemporaryFile
    pairee --cwd-file $tmp.FullName @args
    $dir = Get-Content -Raw -LiteralPath $tmp.FullName
    if ($dir -and $dir -ne $PWD.Path) { Set-Location -LiteralPath $dir }
    Remove-Item -LiteralPath $tmp.FullName
}
```

### Launch Standalone Mode
Run `pairee --standalone` (from a checkout: `cargo xtask dev standalone`) to boot the application in a dedicated console window. `cargo xtask dev` lists the other developer actions.

---

## ⚙️ Configuration & Data Paths

Pairee stores configurations, custom themes, and debug logs in the system's standard directories:

* **Windows:** `%APPDATA%/pairee/config` and `%APPDATA%/pairee/cache`
* **Linux/macOS:** `~/.config/pairee` and `~/.cache/pairee`

Logs are appended to `app.log` in the cache directory, allowing you to troubleshoot without cluttering the TUI. Past 5 MiB the file moves to `app.log.1`. Release builds log at `info` level; run with `RUST_LOG=debug` for more detail.

---

## 📖 Learn More

For deep-dive instructions, design principles, and config options, refer to the following documents:

| Topic | English | Español |
| :--- | :--- | :--- |
| **Project Wiki & Docs** | [DeepWiki](https://deepwiki.com/FittyAr/Pairee) | [DeepWiki](https://deepwiki.com/FittyAr/Pairee) |
| **Docs index & status** | [docs/README.md](docs/README.md) | [docs/README.md](docs/README.md) |
| **Improvement plan** | [IMPROVEMENT_PLAN.md](docs/IMPROVEMENT_PLAN.md) | [IMPROVEMENT_PLAN.md](docs/IMPROVEMENT_PLAN.md) |
| **Full Features Reference** | [Features Manual](help/en/features.md) | [Manual de Funciones](help/es/features.md) |
| **System Architecture** | [Architecture Guide](docs/technical/architecture_en.md) | [Guía de Arquitectura](docs/technical/architecture_es.md) |
| **Configuration & Options** | [User Guide](help/en/user_guide.md) | [Guía de Usuario](help/es/user_guide.md) |

---

## 📄 License

This project is licensed under the GNU General Public License v3. See [LICENSE](LICENSE) for details.
