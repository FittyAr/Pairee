# Guide: Installers and Release Packages

How the release packages are built. Everything runs in
`.github/workflows/release.yml`; the per-target work is in
`.github/scripts/package.sh` (with `package-msix.ps1` for the MSIX), and
`.github/scripts/verify-dist.sh` smoke-tests every package before upload.

---

## 1. Windows: Inno Setup Installer (.exe)

`installer.iss` builds a per-user installer (`{localappdata}\Programs\pairee`,
no UAC prompt) that copies `pairee.exe`, puts `lang/` and `help/` in
`%APPDATA%\pairee\config`, adds the install folder to the user `PATH` once
and removes it again on uninstall. `package.sh` compiles it with the Inno
Setup 6 of the Windows runner, passing everything that varies per build:

```bat
iscc /DAppVersion=X.Y.Z /DSourceDir=target\<triple>\release /DAppArch=x64|arm64 installer.iss
```

The version therefore comes from `Cargo.toml` through the release tag; the
script's own defaults are only for a local build. An `arm64` installer only
installs on ARM64 Windows (`ArchitecturesAllowed`).

---

## 2. Linux: Debian/Ubuntu Package (.deb)

The x86_64 musl build is also packaged with
[`cargo-deb`](https://github.com/kornelski/cargo-deb), configured in
`[package.metadata.deb]` of `Cargo.toml`: the binary goes to `/usr/bin`, the
translations, help pages and keymap presets to `/usr/share/pairee/`, which
Pairee searches after its configuration folder
(`config::paths::get_system_share_dir`). `package.sh` runs
`cargo deb --target x86_64-unknown-linux-musl --no-build` on the release
binary.

---

## 3. Linux: Red Hat/Fedora Package (.rpm)

The same files are packaged with
[`cargo-generate-rpm`](https://github.com/cat-in-136/cargo-generate-rpm),
configured in `[package.metadata.generate-rpm]` of `Cargo.toml`
(`cargo generate-rpm --target x86_64-unknown-linux-musl`). Both tools are
installed as prebuilt binaries (`taiki-e/install-action`) in their latest
release.

---

## 4. Release Signing (minisign)

Every release asset is signed with the **Pairee release key** (minisign /
Ed25519). The self-updater (`src/update/signature.rs`) refuses to install an
update unless the release contains both `<asset>.sha256` **and**
`<asset>.minisig`, the signature verifies against the embedded public key and
its trusted comment names the asset (`file:<asset>`). The install scripts
always check the SHA-256 and require the signature too:

- `install.sh` uses the `minisign` on `PATH`; without it, it downloads the
  official Linux build (`jedisct1/minisign`, version and SHA-256 of the
  tarball pinned in the script) into its temporary folder and verifies with
  it, so `curl | sh` works without installing anything first.
- `install.ps1` downloads the official Windows build of minisign
  (`jedisct1/minisign`, version and SHA-256 of the zip pinned in the script)
  into its temporary folder and verifies with it.

Either can be told to accept the download on its checksum alone
(`install.sh --insecure-skip-signature`, `install.ps1 -InsecureSkipSignature`),
which is weaker: the checksum is served from the same place as the archive.
`install.sh --verify-only` / `install.ps1 -VerifyOnly` download and verify the
latest release and install nothing (a dry run of the download path).

To move to a newer minisign, update both scripts together
(`tests/release_key.rs` checks that the versions agree): in `install.ps1`
`$minisignVersion` and `$minisignZipSha256` (the SHA-256 of
`minisign-<ver>-win64.zip`), in `install.sh` `MINISIGN_VERSION` and
`MINISIGN_TARBALL_SHA256` (of `minisign-<ver>-linux.tar.gz`). Check each
archive against the `.minisig` published with it, made with the minisign
author's key `RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3`).

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
and the updater of those versions only looks for that name. **0.11.0**
(`LEGACY_NAMES_UNTIL` in `xtask/src/assets.rs`) therefore also published each
archive under its old name, with its own `.sha256` and `.minisig`, so 0.10
and older can update to it; the install scripts and the updater fall back to
the old name. Later releases carry only the platform names (an install of
0.10 or older updates to 0.11.0 by hand first, or reinstalls). Delete the code marked
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
