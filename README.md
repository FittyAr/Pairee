# NCRust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

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

---

## 📂 Project Structure

```text
NCRust/
├── Cargo.toml                     # Cargo configuration
├── agents.md                      # AI Developer Guidelines
├── LICENSE                        # MIT License
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

### Prerequisites
Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed.

### Build and Run
```bash
# Clone the repository
git clone https://github.com/FittyAr/NCRust.git
cd NCRust

# Run NCRust in development mode
cargo run

# Build the release binary
cargo build --release
```

### Launch Standalone Mode
You can check launcher script wrappers `run.bat` (Windows) or `run.sh` (Linux/macOS) to boot the application in a dedicated console window.

---

## ⚙️ Configuration & Data Paths

NCRust stores configurations, custom themes, and debug logs in the system's standard directories:

* **Windows:** `%APPDATA%/ncrust/config` and `%APPDATA%/ncrust/cache`
* **Linux/macOS:** `~/.config/ncrust` and `~/.cache/ncrust`

Debug logs are saved to `app.log` in the cache directory, allowing you to troubleshoot without cluttering the TUI.

---

## 📖 Learn More

For deep-dive instructions, design principles, and config options, refer to the following documents:

| Topic | English | Español |
| :--- | :--- | :--- |
| **Full Features Reference** | [Features Manual](help/features_en.md) | [Manual de Funciones](help/features_es.md) |
| **System Architecture** | [Architecture Guide](docs/technical/architecture_en.md) | [Guía de Arquitectura](docs/technical/architecture_es.md) |
| **Configuration & Options** | [User Guide](help/user_guide_en.md) | [Guía de Usuario](help/user_guide_es.md) |

---

## 📄 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
