# Guide: Generating Inno Setup, .deb, and .rpm Installers

This guide outlines how to generate system-native installer packages for Windows and Linux.

---

## 1. Windows: Inno Setup Installer (.exe)

Inno Setup is a script-driven installation builder. It packages the executable and resource directories into a single `.exe` file that sets up directories, adds desktop/start menu shortcuts, and registers PATH environment variables.

### Step A: Create the script `installer.iss` in your repository root
```pascal
; installer.iss
#define AppName "Pairee"
#define AppVersion "0.1.2"
#define AppPublisher "FittyAr"
#define AppURL "https://github.com/FittyAr/Pairee"
#define AppExeName "pairee.exe"

[Setup]
AppId={{D37E8417-C08D-43EC-4FE5-87673A12B57F}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={localappdata}\Programs\pairee
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=target\release
OutputBaseFilename=pairee-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "target\x86_64-pc-windows-msvc\release\pairee.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "lang\*"; DestDir: "{userappdata}\pairee\config\lang"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "help\*"; DestDir: "{userappdata}\pairee\config\help"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"

[Registry]
; Safely append to user PATH environment variable
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{userappdata}\Local\Programs\pairee;{olddata}"; Flags: preservestringtype
```

### Step B: GitHub Actions Integration
Add the setup and compiler step to your `release.yml` for the Windows runner:
```yaml
      - name: Install Inno Setup
        if: matrix.os == 'windows-latest'
        uses: AmrDeveloper/setup-inno-setup@v1

      - name: Build Inno Setup Installer
        if: matrix.os == 'windows-latest'
        run: |
          iscc installer.iss
          echo "ASSET_PATH=target/release/pairee-setup-${{ env.VERSION }}.exe" >> $GITHUB_ENV
```

---

## 2. Linux: Debian/Ubuntu Package (.deb)

We can use `cargo-deb`, a Cargo helper command that compiles the binary and packages it with your resources according to Debian standard layouts.

### Step A: Update `Cargo.toml`
Add the metadata configuration at the bottom of your `Cargo.toml`:
```toml
[package.metadata.deb]
name = "pairee"
maintainer = "FittyAr <10284757+FittyAr@users.noreply.github.com>"
license-file = ["LICENSE", "4"]
depends = "$default"
section = "utils"
priority = "optional"
assets = [
    ["target/x86_64-unknown-linux-musl/release/pairee", "usr/bin/pairee", "755"],
    ["lang/*", "usr/share/pairee/lang/", "644"],
    ["help/*", "usr/share/pairee/help/", "644"],
]
```

### Step B: Code adjustments (Lookup path logic)
Since resources are installed to `/usr/share/pairee/`, we should update the resource lookup logic in `src/config/paths.rs` to scan this path on Linux:
```rust
// In src/config/paths.rs:
pub fn get_system_share_dir() -> Option<PathBuf> {
    #[cfg(not(target_os = "windows"))]
    {
        let share_dir = PathBuf::from("/usr/share/pairee");
        if share_dir.exists() {
            return Some(share_dir);
        }
    }
    None
}
```
Update your `discover_languages` and help file path resolves to include `get_system_share_dir()` as a fallback search path.

### Step C: GitHub Actions Integration
Add the cargo-deb step to the Linux release job:
```yaml
      - name: Install cargo-deb
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: cargo install cargo-deb

      - name: Build Debian Package
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: |
          cargo deb --target x86_64-unknown-linux-musl --no-build
          echo "ASSET_PATH=target/x86_64-unknown-linux-musl/debian/pairee_${{ env.VERSION }}_amd64.deb" >> $GITHUB_ENV
```

---

## 3. Linux: Red Hat/Fedora Package (.rpm)

Similar to Debian, we can use `cargo-generate-rpm`, which is written in Rust, runs on any platform (no system `rpmbuild` required), and reads configuration from `Cargo.toml`.

### Step A: Update `Cargo.toml`
Add the RPM metadata section:
```toml
[package.metadata.generate-rpm]
name = "pairee"
summary = "A sleek, dual-panel terminal file manager"
description = "A modern Rust clone of Norton Commander with dual-panel layout, themes, and full mouse support."
license = "GPLv3"
categories = ["Applications/System"]
assets = [
    { source = "target/x86_64-unknown-linux-musl/release/pairee", dest = "/usr/bin/pairee", mode = "755" },
    { source = "lang/*", dest = "/usr/share/pairee/lang/", mode = "644" },
    { source = "help/*", dest = "/usr/share/pairee/help/", mode = "644" },
]
```

### Step B: GitHub Actions Integration
Add the generation step to the Linux release job:
```yaml
      - name: Install cargo-generate-rpm
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: cargo install cargo-generate-rpm

      - name: Build RPM Package
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: |
          cargo generate-rpm --target x86_64-unknown-linux-musl
          echo "ASSET_PATH=target/x86_64-unknown-linux-musl/generate-rpm/pairee-${{ env.VERSION }}-1.x86_64.rpm" >> $GITHUB_ENV
```
