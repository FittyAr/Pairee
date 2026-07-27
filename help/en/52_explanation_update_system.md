# 💡 Explanation: The Update System and 13 Install Methods

> **Quadrant: EXPLANATION** — *understanding-oriented.*

Pairee's auto-updater is more than a "check GitHub and download" loop.
It detects **how you installed Pairee** and applies the right update
strategy. This page explains the detection, the integrity check, and
why thirteen install methods exist.

---

## 1. The detection phase

When Pairee starts, the `update::detect` module tries to identify the
install method in this order:

| # | Method | How it's detected | Update action |
| --- | --- | --- | --- |
| 1 | **tar.gz direct binary** (Linux) | Binary path lives under `~/.local/bin/pairee` or `/usr/local/bin/pairee`, no package manager owns it. | Download the new tarball, atomic-replace the binary, prompt to restart. |
| 2 | **ZIP direct binary** (Windows) | Binary path lives under `%LOCALAPPDATA%\Programs\pairee\pairee.exe` (or similar), no MSI/EXE installer registry. | Download the new ZIP, write a self-destructing `.bat` helper, exit Pairee, the helper replaces the binary and re-launches. |
| 3 | **Inno Setup installer** (Windows) | Pairee is registered in `Add/Remove Programs` with `Inno Setup` as the publisher, or a registry key named "Inno Setup:". | Download the installer, run it with `/VERYSILENT`, exit Pairee. |
| 4 | **`apt`** (Debian, Ubuntu, derivatives) | `dpkg -S $(which pairee)` returns a `.deb` package; `/var/lib/dpkg/info/pairee.list` exists. | Display the `sudo apt update && sudo apt install pairee` command. |
| 5 | **`dnf` / `yum`** (Fedora, RHEL, CentOS) | `rpm -qf $(which pairee)` returns a package. | Display the `sudo dnf upgrade pairee` command. |
| 6 | **`pacman`** (Arch, Manjaro) | `pacman -Qo $(which pairee)` returns the package. | Display the `sudo pacman -Syu pairee` command. |
| 7 | **`zypper`** (openSUSE) | `zypper se --installed-only pairee` returns the package. | Display the `sudo zypper update pairee` command. |
| 8 | **`nix`** (NixOS, nixpkgs) | Binary path is inside `/nix/store/`. | Display the `nix-env -u pairee` or `nixos-rebuild switch` command. |
| 9 | **`snap`** | `snap list pairee` returns a package. | Display the `sudo snap refresh pairee` command. |
| 10 | **`flatpak`** | `flatpak list | grep pairee` returns a package. | Display the `flatpak update pairee` command. |
| 11 | **`winget`** (Windows) | `winget list | grep pairee` returns a package. | Display the `winget upgrade FittyAr.Pairee` command. |
| 12 | **`scoop`** (Windows) | `scoop list | grep pairee` returns a package. | Display the `scoop update pairee` command. |
| 13 | **`chocolatey`** (Windows) | `choco list | grep pairee` returns a package. | Display the `choco upgrade pairee` command. |

If none of the above match, Pairee assumes the **direct binary** path
and offers to download a tarball/ZIP into the binary's directory.

> The detection happens once per launch and the result is cached in
> memory. You can re-run the detection manually from
> `F9` → `Options` → `Check for updates`.

---

## 2. The integrity check

Every downloaded release ships with a **`.sha256`** file alongside
the binary archive. Pairee:

1. Downloads the new binary archive.
2. Downloads the matching `.sha256` from the same release.
3. Computes the SHA-256 of the downloaded archive locally.
4. Compares the two hashes.

If they differ, the archive is **deleted immediately** and the
update is aborted. A notification toast tells the user the
verification failed. The local `app.log` records the mismatch for
debugging.

This protects against:

- A compromised GitHub Releases upload.
- A network-path MITM (although HTTPS already prevents this; SHA-256
  is the second line of defence).
- A corrupted download (rare, but possible on flaky networks).

> The verify step happens **before** any file is replaced on disk.
> A failed verification never leaves your system in a partial
> state.

---

## 3. The auto-check

If `auto_update_check = true` (the default), Pairee queries GitHub
Releases once at launch. The query is non-blocking and throttled to
once per hour per session. The result is cached for the rest of the
run.

When a newer release is found, a yellow **`▲ UPDATE`** badge appears
in the top-right of the F-key bar. Click it (or use
`F9` → `Options` → `Check for updates`) to open the update dialog.

### The update dialog

The dialog shows:

- The new version.
- The release date.
- The release notes (rendered from the GitHub Release body as
  Markdown).
- The binary size and the SHA-256 (click to copy).
- A **Download and apply** button (or the package-manager command
  the user should run).

If the user clicks **Download and apply** (or the package-manager
button is the only option), Pairee walks the chosen path:

- **Direct binary**: download, verify, replace, prompt to restart.
- **Inno Setup**: download, run silently, exit.
- **Package manager**: just show the command; the user pastes it in
  their shell.

### Dismissing an update

If the user dismisses the badge, Pairee writes the version string
to `dismissed_update_version` in `settings.toml`. Future launches do
not re-notify for that version. To re-enable notifications, clear the
field by hand.

---

## 4. Why thirteen methods, not one

A single "download and replace" approach is fine for users who
installed Pairee via the install script, but it is **wrong** for
users on package-managed distros:

- A direct-binary update bypasses the package manager, so `apt` /
  `dnf` / `pacman` will report the file as modified and may
  overwrite it on the next system update.
- On **NixOS**, files under `/nix/store/` are immutable; trying to
  replace them silently will fail.
- On **Windows**, an installed EXE has a different update path
  (MSI/EXE installer) than a portable ZIP.
- **Snap** and **Flatpak** sandboxes forbid writing into their
  mount points; updates must go through the sandbox manager.

Showing the user the **right command for their install method** is
both safer and faster than trying to be clever on their behalf.

---

## 5. The update flow, end-to-end

```
launch
  └─▶ update::detect         (figure out install method)
        └─▶ GitHub Releases   (if auto_update_check)
              └─▶ newer?     (compare semver)
                    └─▶ yes  → render badge
                              └─▶ user clicks badge
                                    └─▶ update::downloader
                                          ├─▶ download archive
                                          ├─▶ download .sha256
                                          ├─▶ verify hash
                                          └─▶ update::installer
                                                ├─▶ direct: replace + restart
                                                ├─▶ Inno Setup: run installer silently
                                                └─▶ package manager: show command
```

---

## 6. Where to look in the code

- `src/update/detect.rs` — install-method detection.
- `src/update/downloader.rs` — download + SHA-256 verify.
- `src/update/installer.rs` — apply the right update.
- `src/update/checker.rs` — query GitHub Releases.

---

## Where to go next

- Update workflow: [`29_howto_install_build_update`](29_howto_install_build_update.md)
- Configuration: [`41_reference_configuration`](41_reference_configuration.md)
