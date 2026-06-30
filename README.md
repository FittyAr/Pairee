# Pairee

> Tu mundo, en dos paneles. (Your world, in two panels.)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/FittyAr/Pairee)
![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/FittyAr/Pairee?utm_source=oss&utm_medium=github&utm_campaign=FittyAr%2FPairee&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

## Pairee - ⚡ Modern Dual-Panel Terminal File Manager

Pairee is a modern, highly modular, and cross-platform terminal file manager inspired by the classic **Norton Commander** and **Far Manager**. Built in Rust utilizing `ratatui` and `crossterm`, it aims to provide an efficient, fast, and extensible file management experience directly within your console.

- 🚀 **Full Asynchronous Support**: All long-running filesystem operations (Copy, Move, Wipe, Delete) run on background threads (`tokio`), keeping the UI perfectly responsive.
- 💪 **Async Task Scheduling**: Real-time progress tracking, progress bar popups, and task cancellation for concurrent workers.
- 🎨 **Visual Themes & Layouts**: Customizable theme loader supporting Slate, Blue, and other custom styles, alongside responsive layout division.
- ⚙️ **Flexible Keybindings**: Dynamic keybinding resolver featuring preset profiles for Classic Norton (F1-F10), Vim navigation, and Modern keys.
- 🔌 **Extensible Plugin System (Planned)**: Future support for concurrent Lua-based plugins to add custom file previewers, search adapters, and UI widgets.
- 🧰 **Advanced Utilities**: Built-in file search by name/content, folder comparison, custom user commands menu, OS process manager, and file attributes viewer.
- 📦 **Smart Auto-Updates**: Secure checks supporting auto-detection across 13 installation methods with SHA-256 verification.
- 🌐 **Centralized Translations**: Core translation engine supporting English, Spanish, and easily extendable to new languages.

---

## Project Status

Public beta. Stable enough to be used as a daily driver. Pairee is in active development, and suggestions or contributions are highly welcome.

---

## 📂 Project Structure

```text
Pairee/
├── Cargo.toml                     # Cargo configuration
├── install.sh                     # Linux installer script (curl-compatible)
├── install.ps1                    # Windows installer script (PowerShell-compatible)
├── LICENSE                        # GNU GPL v3 License
├── README.md                      # English documentation index (This file)
├── README.es.md                   # Spanish documentation index
├── .agents/                       # AI Developer Guidelines and Custom Skills
├── docs/                          # Developer documentation
│   └── technical/
│       ├── architecture_en.md     # Architecture & codebase design (English)
│       └── architecture_es.md     # Architecture & codebase design (Spanish)
├── help/                          # User help documentation (loaded in-app via F1)
│   ├── features_en.md             # In-depth feature manual (English)
│   ├── features_es.md             # In-depth feature manual (Spanish)
│   ├── user_guide_en.md           # Configuration & customization guide (English)
│   └── user_guide_es.md           # Configuration & customization guide (Spanish)
└── src/                           # Source code
    ├── main.rs                    # Application entry point
    ├── app/                       # Event loops, actions, and state management
    ├── config/                    # TOML settings, themes, and translations
    ├── fs/                        # Filesystem actions and background task channels
    ├── keybindings/               # Input mappings and resolver engine
    ├── ui/                        # Ratatui panels, menus, and popups
    └── terminal/                  # Raw terminal screen controller & backend setup
```

---

## 🛠️ Quick Start

### Installation (Quick Installers)

Pairee provides precompiled binaries built automatically via GitHub Actions (supporting Windows MSVC, Linux GNU, and static Linux MUSL). You can install them instantly via command line:

* **Linux (statically linked, standalone binary):**
  * **Standard Release:**
    ```bash
    curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
    ```
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
  * **Build from Source (Debug/Source Mode):**
    ```powershell
    irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex -Arguments debug
    ```
  * **Uninstall:**
    ```powershell
    irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex -Arguments uninstall
    ```

### Build from Source

#### Prerequisites
Make sure you have [Rust](https://www.rust-lang.org/tools/install) (version 1.70 or higher) installed.

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

### Launch Standalone Mode
You can use the launcher script wrappers `run.bat` (Windows) or `run.sh` (Linux/macOS) to boot the application in a dedicated console window.

---

## ⚙️ Configuration & Data Paths

Pairee stores configurations, custom themes, and debug logs in the system's standard directories:

* **Windows:** `%APPDATA%/pairee/config` and `%APPDATA%/pairee/cache`
* **Linux/macOS:** `~/.config/pairee` and `~/.cache/pairee`

Debug logs are saved to `app.log` in the cache directory, allowing you to troubleshoot without cluttering the TUI.

---

## 📖 Learn More

For deep-dive instructions, design principles, and config options, refer to the following documents:

| Topic | English | Español |
| :--- | :--- | :--- |
| **Project Wiki & Docs** | [DeepWiki](https://deepwiki.com/FittyAr/Pairee) | [DeepWiki](https://deepwiki.com/FittyAr/Pairee) |
| **Full Features Reference** | [Features Manual](help/features_en.md) | [Manual de Funciones](help/features_es.md) |
| **System Architecture** | [Architecture Guide](docs/technical/architecture_en.md) | [Guía de Arquitectura](docs/technical/architecture_es.md) |
| **Configuration & Options** | [User Guide](help/user_guide_en.md) | [Guía de Usuario](help/user_guide_es.md) |

---

## 📄 License

This project is licensed under the GNU General Public License v3. See [LICENSE](LICENSE) for details.
