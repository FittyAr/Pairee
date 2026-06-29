# Pairee

> Tu mundo, en dos paneles. (Your world, in two panels.)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Wiki: DeepWiki](https://img.shields.io/badge/Wiki-DeepWiki-green.svg)](https://deepwiki.com/FittyAr/Pairee/)
![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/FittyAr/Pairee?utm_source=oss&utm_medium=github&utm_campaign=FittyAr%2FPairee&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

A modern, highly modular, cross-platform terminal file manager inspired by the classic **Norton Commander** and **Far Manager**. Built in Rust utilizing `ratatui` and `crossterm`.

---

## 🚀 Key Features

* **Dual-Panel Interface:** Classic, efficient two-panel layout to navigate directories and perform bulk file operations.
* **Dual-Mode Startup:** Run as an in-terminal app or launch a standalone, optimized terminal window directly.
* **Async Background Operations:** Long-running tasks (Copy, Move, Wipe, Delete) are processed on a background thread pool (`tokio`), keeping the UI fully responsive with real-time progress bars.
* **Flexible Keybinding Resolver:** Built-in profiles for Classic Norton, Vim, and Modern navigation keys.
* **High Extensibility:** Modular layout conforming to the Single Responsibility Principle, theme loaders, and customizable plugins.
* **Localization & Themes:** Centralized translation lookup engine supporting English, Spanish, and custom themes (Slate, Blue, etc.).
* **Advanced Features:** File search by content/name, folder comparison, custom user commands menu, OS process manager, and attributes viewer.
* **Smart Auto-Update System:** Built-in secure updates that auto-detect the installation source across 13 methods (like package managers or direct builds) and download/install with SHA-256 checks.

---

## 📂 Project Structure

```text
Pairee/
├── Cargo.toml                     # Cargo configuration
├── agents.md                      # AI Developer Guidelines
├── install.sh                     # Linux installer script (curl-compatible)
├── install.ps1                    # Windows installer script (PowerShell-compatible)
├── LICENSE                        # GNU GPL v3 License
├── README.md                      # English documentation index (This file)
├── README.es.md                   # Spanish documentation index
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

NCRust provides precompiled binaries built automatically via GitHub Actions (supporting Windows MSVC, Linux GNU, and static Linux MUSL). You can install them instantly via command line:

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
Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed.

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
You can check launcher script wrappers `run.bat` (Windows) or `run.sh` (Linux/macOS) to boot the application in a dedicated console window.

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
| **Project Wiki & Docs** | [DeepWiki](https://deepwiki.com/FittyAr/Pairee/) | [DeepWiki](https://deepwiki.com/FittyAr/Pairee/) |
| **Full Features Reference** | [Features Manual](help/features_en.md) | [Manual de Funciones](help/features_es.md) |
| **System Architecture** | [Architecture Guide](docs/technical/architecture_en.md) | [Guía de Arquitectura](docs/technical/architecture_es.md) |
| **Configuration & Options** | [User Guide](help/user_guide_en.md) | [Guía de Usuario](help/user_guide_es.md) |

---

## 📄 License

This project is licensed under the GNU General Public License v3. See [LICENSE](LICENSE) for details.
