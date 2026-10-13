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

---

## 4. Release Signing (minisign)

Every release asset is signed with the **Pairee release key** (minisign /
Ed25519). The self-updater (`src/update/signature.rs`) refuses to install an
update unless the release contains both `<asset>.sha256` **and**
`<asset>.minisig`, the signature verifies against the embedded public key and
its trusted comment names the asset (`file:<asset>`). The install scripts
always check the SHA-256 and require the signature too:

- `install.sh` uses the `minisign` on `PATH`; without it, it stops.
- `install.ps1` downloads the official Windows build of minisign
  (`jedisct1/minisign`, version and SHA-256 of the zip pinned in the script)
  into its temporary folder and verifies with it.

Either can be told to accept the download on its checksum alone
(`install.sh --insecure-skip-signature`, `install.ps1 -InsecureSkipSignature`),
which is weaker: the checksum is served from the same place as the archive.
`install.sh --verify-only` / `install.ps1 -VerifyOnly` download and verify the
latest release and install nothing (a dry run of the download path).

To move `install.ps1` to a newer minisign, update `$minisignVersion` and
`$minisignZipSha256` together (the SHA-256 of `minisign-<ver>-win64.zip`;
check it against the `.minisig` published with it, made with the minisign
author's key `RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaE`).

**Public key** (key id `B478F4237C0DE585`), embedded in
`src/update/signature.rs` (`RELEASE_PUBLIC_KEY`), `install.sh` and
`install.ps1` (`tests/release_key.rs` checks that they agree):

```
untrusted comment: minisign public key: B478F4237C0DE585
RWSF5Q18I/R4tADVZ5LQzgP2gRzPD/yzWj0p5kw13d6g4+Ycwbn27Fm6
```

Manual verification of a downloaded asset:

```bash
minisign -V -P RWSF5Q18I/R4tADVZ5LQzgP2gRzPD/yzWj0p5kw13d6g4+Ycwbn27Fm6 -m pairee-X.Y.Z-linux-x64-musl.tar.gz
```

### Release asset names

Archives are named after the platform, not the Rust target triple:

| Target triple | Archive |
|---|---|
| `x86_64-unknown-linux-gnu` | `pairee-X.Y.Z-linux-x64-gnu.tar.gz` |
| `x86_64-unknown-linux-musl` | `pairee-X.Y.Z-linux-x64-musl.tar.gz` |
| `aarch64-unknown-linux-musl` | `pairee-X.Y.Z-linux-arm64-musl.tar.gz` |
| `x86_64-apple-darwin` | `pairee-X.Y.Z-macos-x64.tar.gz` |
| `aarch64-apple-darwin` | `pairee-X.Y.Z-macos-arm64.tar.gz` |
| `x86_64-pc-windows-msvc` | `pairee-X.Y.Z-windows-x64.zip` |
| `aarch64-pc-windows-msvc` | `pairee-X.Y.Z-windows-arm64.zip` |

plus `pairee-setup-X.Y.Z-<x64|arm64>.exe`, `pairee-X.Y.Z-<x64|arm64>.msix`
and the `.deb` / `.rpm` of the x64 musl build. The table lives in
`src/update/assets.rs` only: the updater reads it directly and the packaging
scripts through `cargo xtask asset-names <triple> <vX.Y.Z>`;
`tests/release_assets.rs` checks the names spelled out in `install.sh`,
`install.ps1` and the workflows.

**Transition.** Up to 0.10 the archives were `pairee-vX.Y.Z-<triple>.<ext>`,
and the updater of those versions only looks for that name. Releases up to
**0.12.0** (`LEGACY_NAMES_UNTIL` in `xtask/src/assets.rs`) therefore also
publish each archive under its old name, with its own `.sha256` and
`.minisig`; the install scripts and the updater fall back to the old name.
From 0.13.0 the copies stop on their own; then delete the code marked
`TRANSITION` (`legacy_archive_name`, the `LEGACY` handling in
`.github/scripts/package.sh` / `verify-dist.sh`, the fallbacks in
`install.sh`, `install.ps1` and `src/update/downloader.rs`).

### Required GitHub secrets

| Secret | Content |
|--------|---------|
| `MINISIGN_SECRET_KEY` | Full contents of the minisign secret key file (both lines). |
| `MINISIGN_PASSWORD` | Optional. Only if the secret key is password-protected. |

The `Draft release` job in `.github/workflows/release.yml` installs
`minisign` and runs `.github/scripts/sign.sh`, which signs every file in
`dist/` (except `.sha256` / `.minisig`) with the trusted comment
`file:<asset> version:<tag>` and checks every signature against the public
key embedded in `src/update/signature.rs`; the job uploads the `.minisig`
files with the release. Packaging and checksums are
`.github/scripts/package.sh` (with `package-msix.ps1`) and
`.github/scripts/checksums.sh`. If `MINISIGN_SECRET_KEY` is missing, the
job fails instead of publishing unsigned assets.

### Key custody

- The secret key is **never** stored in the repository (`*.key` is ignored).
  The maintainer keeps it offline (e.g. `~/.pairee-release-keys/`) and in the
  GitHub secret only.
- The current key was generated without a password so CI can use it
  non-interactively; GitHub encrypts secrets at rest. To add a password,
  re-encrypt it (`minisign -C`) and set `MINISIGN_PASSWORD`.
- **Rotation:** generate a new pair (`minisign -G` or `rsign generate`),
  update `RELEASE_PUBLIC_KEY`, `install.sh`, `install.ps1` and this guide,
  replace the secret, and sign the release that introduces the new key with
  the *old* key, so existing installations can still verify it.
