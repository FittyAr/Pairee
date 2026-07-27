# 🔧 How-To: Install, Build, and Update

> **Quadrant: HOW-TO** — *problem-oriented.*

Three everyday needs share this page: **installing** Pairee the first
time, **building** it from source, and **updating** an existing
installation.

---

## Install via the quick script (recommended)

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
```

The script:

1. Detects your platform and architecture.
2. Downloads the latest release from GitHub.
3. Verifies the SHA-256 hash against `*.sha256` from the release.
4. Installs the binary (`/usr/local/bin/pairee` or
   `~/.local/bin/pairee`, whichever is writable).
5. Installs the `lang/`, `help/`, and `keymaps/` folders under
   `/usr/share/pairee/` (or `~/.local/share/pairee/`).

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
```

The PowerShell script mirrors the bash script: download, SHA-256
verify, install to `%LOCALAPPDATA%\Programs\pairee`.

### Build from source via the script

If you want the latest unreleased build (or no release exists for your
platform), pass the `debug` argument:

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- debug
```

This clones the repo, runs `cargo build`, and installs the resulting
binary.

### Uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh -s -- uninstall
```

Or on Windows:

```powershell
irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex -Arguments uninstall
```

---

## Build from source manually

### Prerequisites

- **Rust 1.70 or newer** ([install instructions](https://www.rust-lang.org/tools/install))
- A C toolchain (only for some optional features)
- Git

### Steps

```bash
# Clone
git clone https://github.com/FittyAr/Pairee.git
cd Pairee

# Debug build (fast, includes debug symbols)
cargo build

# Release build (optimised, stripped of debug logs)
cargo build --release
```

The compiled binary is at:

- `target/debug/pairee` (or `pairee.exe`)
- `target/release/pairee` (or `pairee.exe`)

### Run from a dedicated console

Two convenience scripts open Pairee in its own terminal window:

- Linux / macOS: `./run.sh`
- Windows: `run.bat`

---

## Update Pairee

### Auto-detected in-app

If `auto_update_check = true` (the default), Pairee queries GitHub
Releases at startup. When a new release is published, a yellow
**`▲ UPDATE`** badge appears in the top-right corner.

1. Click the badge (or `F9` → `Options` → `Check for updates`).
2. The update dialog shows the release notes and size.
3. Pairee detects **how it was installed** and applies the right
   action:

   | Install method | What happens |
   | --- | --- |
   | **tar.gz / ZIP direct binary** | Download the new release, verify SHA-256, atomic-replace the binary, prompt to restart. |
   | **Windows Inno Setup installer** | Download the installer, run it silently (`/VERYSILENT`), exit Pairee. |
   | **`apt` / `dnf` / `pacman` / `nix`** | Display the exact `sudo apt update && sudo apt install pairee` command for you to run. |
   | **`winget` / `scoop` / `chocolatey`** | Display `winget upgrade Pairee` (or the equivalent for your package manager). |
   | **`snap` / `flatpak`** | Display the corresponding package-manager command. |

4. The downloader always fetches the matching `*.sha256` and refuses
   to install if the hash does not match.

### Manually ignore a release

If you want to stay on the current version despite a new release,
dismiss the badge. Pairee writes the ignored version to
`dismissed_update_version` in `settings.toml`. Clear that field to
re-enable notifications for that version.

### Full design notes

See [`52_explanation_update_system`](52_explanation_update_system.md)
for the 13 install methods Pairee can detect and the rationale behind
the SHA-256 verification.

---

## Configuration and data locations

| What | Windows | Linux / macOS |
| --- | --- | --- |
| Config (themes, presets, history) | `%APPDATA%\pairee\config\` | `~/.config/pairee/` |
| Cache (plugin locks, update badges) | `%APPDATA%\pairee\cache\` | `~/.cache/pairee/` |
| Debug log | `%APPDATA%\pairee\cache\app.log` | `~/.cache/pairee/app.log` |

The first time Pairee starts, it creates these directories and
populates them with sensible defaults.

---

## Where to go next

- Why 13 install methods, not 1: [`52_explanation_update_system`](52_explanation_update_system.md)
- Configuration reference: [`41_reference_configuration`](41_reference_configuration.md)
