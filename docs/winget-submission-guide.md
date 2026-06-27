# WinGet Submission Guide — Pairee

> **Identifier:** `FittyAr.Pairee`  
> **Manifest version used:** `1.9.0`  
> **Workflows:** [`.github/workflows/winget.yml`](../.github/workflows/winget.yml) · [`.github/workflows/release.yml`](../.github/workflows/release.yml)  
> **Local manifests:** [`manifests/winget/`](../manifests/winget/)

---

## Overview

Pairee uses a **two-stage WinGet strategy**:

| Stage | When | How |
|---|---|---|
| **Initial submission** | First release only | Manual — `wingetcreate` CLI or direct PR |
| **Subsequent updates** | Every published GitHub release | Automatic — `winget.yml` GitHub Action |

> [!IMPORTANT]
> Automated GitHub Actions can only **update an existing package**. The very first version must always be submitted manually via Pull Request to [`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs).

---

## Prerequisites (Windows Machine)

Before starting, make sure you have:

- [ ] `winget` installed (comes with Windows 10/11 or App Installer from the Microsoft Store).
- [ ] A **GitHub account** (`FittyAr`) with a fork of [`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs).
- [ ] A **GitHub Classic PAT** with `public_repo` scope (needed to push to your fork and open PRs). Create one at: **GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)**.
- [ ] The published GitHub Release for `v0.6.0` with both installers uploaded:
  - `pairee-setup-0.6.0-x64.exe`
  - `pairee-setup-0.6.0-arm64.exe`

---

## Step 1 — Verify the GitHub Release Is Published

The release workflow ([`release.yml`](../.github/workflows/release.yml)) creates a **draft** release automatically when a `v*` tag is pushed. You must **manually publish it** from GitHub's Releases page.

1. Go to `https://github.com/FittyAr/Pairee/releases`.
2. Click **Edit** on the draft release for `v0.6.0`.
3. Review the release notes (auto-extracted from `CHANGELOG.md`).
4. Verify both Windows installers are attached:
   - `pairee-setup-0.6.0-x64.exe` + its `.sha256` checksum file.
   - `pairee-setup-0.6.0-arm64.exe` + its `.sha256` checksum file.
5. Click **Publish release**.

> [!NOTE]
> Publishing the release also triggers `winget.yml` automatically — but only **after** the package already exists in `winget-pkgs`. For the very first submission this automatic PR will fail because the package does not exist yet.

---

## Step 2 — Calculate SHA-256 Hashes

Download both installers from the release and calculate their hashes locally on a Windows machine:

```powershell
# Download (or navigate to your local build artifacts)
$x64   = "pairee-setup-0.6.0-x64.exe"
$arm64 = "pairee-setup-0.6.0-arm64.exe"

(Get-FileHash $x64   -Algorithm SHA256).Hash
(Get-FileHash $arm64 -Algorithm SHA256).Hash
```

Keep both hashes on hand — you will need them in the manifest files.

---

## Step 3 — First Submission (Choose One Method)

### Option A — `wingetcreate` CLI (Recommended)

This tool automates the PR creation for you.

```powershell
# 1. Install the tool
winget install Microsoft.WingetCreate

# 2. Reopen the terminal, then run:
wingetcreate new "https://github.com/FittyAr/Pairee/releases/download/v0.6.0/pairee-setup-0.6.0-x64.exe"
```

Answer the interactive prompts:

| Field | Value |
|---|---|
| Package Identifier | `FittyAr.Pairee` |
| Publisher | `FittyAr` |
| Package Name | `Pairee` |
| Version | `0.6.0` |
| License | `GPLv3` |
| Installer Type | `inno` |
| Silent switch | *(press Enter — Inno Setup is handled automatically)* |

When asked to submit, enter your **GitHub PAT**. The tool will:
1. Fork `microsoft/winget-pkgs` (or use your existing fork).
2. Generate the manifests under `manifests/f/FittyAr/Pairee/0.6.0/`.
3. Push a branch and open a Pull Request automatically.

> [!WARNING]
> Running `wingetcreate new` with only the x64 URL will omit the `arm64` installer architecture. Additionally, ensure you select `GPLv3` license, as selecting another license (like MIT) will cause validation issues when compared against the code repository.

---

### Option B — Manual Pull Request & Fixing an Existing PR

Use this method if you prefer full control over the manifest files, or **if you need to fix a PR that has already been opened** (such as [PR #394315](https://github.com/microsoft/winget-pkgs/pull/394315)).

#### Step B.1 — Update local manifest files
We have corrected and updated the manifest files under `manifests/f/FittyAr/Pairee/0.6.0/`:
- `FittyAr.Pairee.installer.yaml`: Includes both `x64` and `arm64` URLs and correct SHA-256 hashes.
- `FittyAr.Pairee.locale.en-US.yaml`: Corrected license to `GPLv3` and enriched details.
- `FittyAr.Pairee.locale.es-ES.yaml`: Included Spanish translation file (version `1.12.0`).

#### Step B.2 — Clone your winget-pkgs fork (Optimized Clone)
The `microsoft/winget-pkgs` repository is massive (gigabytes of history). To avoid downloading the entire repository, you can clone **only** the branch created by `wingetcreate` from your fork using a single-branch clone. 

Look at the Pull Request header on GitHub: it will show `from FittyAr:<BRANCH_NAME>`. Clone that specific branch:
```powershell
# In your case, the branch name is FittyAr.Pairee-0.6.0-cf869850-7438-42b9-a3eb-0ded6559bdc6
git clone --branch FittyAr.Pairee-0.6.0-cf869850-7438-42b9-a3eb-0ded6559bdc6 --single-branch https://github.com/FittyAr/winget-pkgs.git
cd winget-pkgs
```
*(This is extremely fast and will only download a few megabytes).*

#### Step B.3 — Verify you are on the correct branch
Since you cloned the branch directly, you will already be on `FittyAr.Pairee-0.6.0-cf869850-7438-42b9-a3eb-0ded6559bdc6`. You can verify this by running:
```powershell
git branch
```

#### Step B.4 — Copy the corrected local manifests into the fork
```powershell
# Copy the 4 correct files from your NCRust repo to the fork
Copy-Item -Path "..\NCRust\manifests\f\FittyAr\Pairee\0.6.0\*.yaml" -Destination "manifests\f\FittyAr\Pairee\0.6.0\" -Force
```

#### Step B.5 — Commit and push to update the PR
```powershell
git add manifests\f\FittyAr\Pairee\0.6.0
git commit -m "Fix license to GPLv3, add ARM64 installer, and add Spanish locale"
git push origin FittyAr.Pairee-0.6.0-cf869850-7438-42b9-a3eb-0ded6559bdc6
```
*This will automatically update the open Pull Request on `microsoft/winget-pkgs` and trigger re-validation.*

---

## Step 4 — PR Review Process

The `microsoft/winget-pkgs` bot (`wingetbot`) and system policy service will run automated validation checks on your PR. Common requirements:

- **Contributor License Agreement (CLA)**: Since `microsoft/winget-pkgs` is managed by Microsoft, first-time contributors must sign a Contributor License Agreement. If you see the `license/cla` check queued or failed, look for the comment by the `microsoft-github-policy-service` bot and reply with a comment on the PR containing:
  ```text
  @microsoft-github-policy-service agree
  ```
  This is required before any maintainers or automated tools can merge your PR.
- **Installer Validation**: The installer URL must be publicly accessible and the computed SHA-256 must match exactly.
- **Silent Installation**: The installer must run completely silently using the specified installer flags.
- **Package ID**: Must follow the standard `Publisher.AppName` pattern (✅ `FittyAr.Pairee`).

> [!NOTE]
> First-time submissions typically require a human review from the winget-pkgs maintainers in addition to the automated checks. Review times vary from a few hours to a few days.

Once the PR is merged, Pairee will be publicly installable via:

```powershell
winget install FittyAr.Pairee
```

---

## Step 5 — Set Up Automated Updates

After the first PR is merged, every future release is handled automatically by [`winget.yml`](../.github/workflows/winget.yml) using the [`vedantmgoyal9/winget-releaser@v2`](https://github.com/vedantmgoyal9/winget-releaser) action.

### One-time setup

1. **Create a Classic PAT** at GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic):
   - Scope: `public_repo`
   - Recommended expiration: 1 year (renew annually).

2. **Keep your fork alive**: the action pushes to `FittyAr/winget-pkgs` on your behalf. Do not delete the fork.

3. **Add the secret to the Pairee repository**:
   - Go to: `https://github.com/FittyAr/Pairee/settings/secrets/actions`
   - Click **New repository secret**
   - Name: `WINGET_TOKEN`
   - Value: your Classic PAT

### How it works for every subsequent release

```
git tag vX.Y.Z  →  push tag
    └─► release.yml runs
            ├─► Builds all targets (Linux x64/musl/ARM64, Windows x64/ARM64)
            ├─► Packages .zip / .tar.gz, .deb, .rpm, Inno Setup .exe
            └─► Creates DRAFT GitHub Release with all binaries attached
    └─► Maintainer reviews and clicks "Publish release"
            └─► winget.yml triggers on [published]
                    └─► vedantmgoyal9/winget-releaser detects new .exe files
                    └─► Computes SHA-256 hashes
                    └─► Pushes branch to FittyAr/winget-pkgs
                    └─► Opens PR to microsoft/winget-pkgs automatically
```

The `installers-regex` in `winget.yml` is configured as `'^pairee-setup-.*\.exe$'`, which matches:
- `pairee-setup-0.6.0-x64.exe`
- `pairee-setup-0.6.0-arm64.exe`

---

## Installation Helper Shells (run.bat & run.sh)

To simplify developer workflow and allow quick testing of the deployed WinGet packages, both [`run.bat`](../run.bat) and [`run.sh`](../run.sh) have been updated with option **Install/Upgrade via WinGet**.

Selecting this option opens a submenu:

```text
==========================================
       Install/Upgrade via WinGet
==========================================
  1. Install Pairee (Auto-detect architecture)
  2. Install Pairee (Force x64)
  3. Install Pairee (Force ARM64)
  4. Upgrade Pairee to latest version
  5. Uninstall Pairee
  6. Back to main menu
==========================================
```

### Script Behaviors:
- **`run.bat` (Windows)**: Directly calls the native `winget` command.
- **`run.sh` (Linux/Cross-platform)**: Dynamically checks if `winget` or `winget.exe` is available in the current path. If running under Windows environments (such as MSYS2, Git Bash, or Cygwin), it invokes the underlying executable. If running on a native Linux system where `winget` is unsupported, it gracefully alerts the user with helper instructions.

---

## Manifest File Reference

All manifest files live in [`manifests/winget/`](../manifests/winget/) and are kept in sync with the current release.

| File | Manifest type | Purpose |
|---|---|---|
| `FittyAr.Pairee.yaml` | `version` | Root version manifest |
| `FittyAr.Pairee.installer.yaml` | `installer` | Installer URLs, hashes, architecture, installer type |
| `FittyAr.Pairee.locale.en-US.yaml` | `defaultLocale` | English metadata (required) |
| `FittyAr.Pairee.locale.es-ES.yaml` | `locale` | Spanish metadata (additional locale) |

When updating for a new release, change `PackageVersion` and `InstallerUrl` in all four files, and update `InstallerSha256` values in the installer manifest.

---

## Quick Reference — Checklist

### First release (v0.6.0)

- [ ] Push tag `v0.6.0` → `release.yml` builds and creates a draft release.
- [ ] Go to GitHub Releases, verify both `.exe` files are attached.
- [ ] Click **Publish release**.
- [ ] Download installers locally and calculate SHA-256 hashes.
- [ ] Choose submission method (Option A with `wingetcreate` is fastest).
- [ ] Submit PR to `microsoft/winget-pkgs`.
- [ ] Reply to the Microsoft policy bot to sign the **CLA** (`@microsoft-github-policy-service agree`).
- [ ] Wait for validations to pass and PR to be merged.
- [ ] Once merged, add `WINGET_TOKEN` secret to the Pairee repository.

### Every subsequent release

- [ ] Push tag `vX.Y.Z` → `release.yml` builds a draft.
- [ ] Publish the draft release on GitHub.
- [ ] `winget.yml` automatically opens a PR to `microsoft/winget-pkgs`.
- [ ] Update local manifests in `manifests/winget/` to reflect the new version (for reference tracking).

---

## Troubleshooting

| Error Label / Issue | Cause / Meaning | Solution |
|---|---|---|
| **`Needs-CLA`** | Contributor License Agreement is not signed. | Reply to the bot comment in the PR with `@microsoft-github-policy-service agree`. |
| **`URL-Validation-Error`** | One of the URLs in the manifests returned a 404 or is inaccessible. | Verify that all URLs (`LicenseUrl`, `PrivacyUrl`, `PublisherUrl`, etc.) are correct. (e.g., ensure you use `/blob/master/LICENSE` instead of `main` if the default branch is `master`). |
| **`Manifest-Validation-Error`** | Syntax error or schema mismatch in YAML files. | Run `winget validate <path-to-manifest>` to debug and resolve schema syntax errors. |
| **`Error-Hash-Mismatch`** / **`Binary-Validation-Error`** | The computed installer hash does not match `InstallerSha256` in the manifest. | Download the installer from the release URL and compute its hash using `Get-FileHash <file> -Algorithm SHA256`. Update the hash in `FittyAr.Pairee.installer.yaml`. |
| `wingetcreate` not found after install | Terminal PATH was not refreshed. | Close and reopen the terminal to reload the environment PATH. |
| `winget.yml` fails on new release | `WINGET_TOKEN` secret is missing or expired. | Create a classic GitHub PAT with `public_repo` scope and save it in repository secrets as `WINGET_TOKEN`. |
| PR blocked on first release | Automated updates only work for existing packages. | Perform the first submission manually (either via `wingetcreate` or a manual PR). |
| `winget install` returns "No package found" | CDN cache is refreshing. | The PR may not be merged yet, or the CDN cache is updating (can take up to 2-3 hours after merge). |
| Inno Setup installer silently fails | Elevation prompts or silent flags mismatch. | Verify that `installer.iss` uses a valid `AppId` GUID and that the installer does not prompt for admin privileges if launched silently. |

---

*Keep this document updated when preparing each new release. Update version numbers, URLs, and hashes accordingly.*
