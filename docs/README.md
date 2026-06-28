# WinGet Integration Guide for Pairee

This guide describes how the official WinGet manifests for **Pairee** are managed. To submit this application to the Windows Package Manager (WinGet) and enable automatic releases, please follow these steps:

---

## 1. Initial Submission (Manual Action Required)

Since automated GitHub Actions can only update *existing* packages, the first release must be submitted manually:

### Option A: Using the `wingetcreate` CLI (Recommended)

1. On a Windows machine, open a terminal (PowerShell/CMD) and install the official WinGet manifest creator tool:
   ```powershell
   winget install Microsoft.WingetCreate
   ```
2. Close and reopen the terminal to refresh your path, then run `wingetcreate new` targeting the URL of your published setup executable:
   ```powershell
   wingetcreate new "https://github.com/FittyAr/Pairee/releases/download/v0.5.1/pairee-setup-0.5.1-x64.exe"
   ```
3. Follow the interactive prompts:
   - **Package ID:** `FittyAr.Pairee`
   - **Publisher Name:** `FittyAr`
   - **Package Name:** `Pairee`
   - **Version:** `0.5.1`
   - **License:** `GPLv3`
   - **Installer Type:** `inno`
   - **Silent Install Switch:** WinGet handles `inno` silent flags automatically, you can press enter to skip.
4. When prompted to submit, provide your **GitHub Personal Access Token (PAT)** with `public_repo` scope. This will automatically fork `microsoft/winget-pkgs`, generate the manifests, push a branch, and open a Pull Request.

### Option B: Manual Pull Request (Using These Files)

1. Fork the [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) repository on GitHub.
2. Clone your fork locally.
3. Compute the SHA-256 hashes of the built Windows installers for the current release:
   - x64: `pairee-setup-0.5.1-x64.exe`
   - arm64: `pairee-setup-0.5.1-arm64.exe`
   You can calculate hashes using PowerShell:
   ```powershell
   Get-FileHash .\pairee-setup-0.5.1-x64.exe
   ```
4. Open the installer manifest (located under `manifests/f/FittyAr/Pairee/<version>/FittyAr.Pairee.installer.yaml`) and replace the SHA256 placeholders (`000000...`) with the actual computed hashes.
5. Create a folder in your cloned fork at:
   `manifests/f/FittyAr/Pairee/<version>/`
6. Copy the four YAML files from the version folder (e.g. `manifests/f/FittyAr/Pairee/<version>/`) into that folder.
7. Commit, push to a branch on your fork, and open a Pull Request to `microsoft/winget-pkgs/master`.

---

## 2. Setting Up Automated Updates (GitHub Actions)

Once the first version is approved and merged into `microsoft/winget-pkgs`, all subsequent versions will be updated automatically when you publish a new release on GitHub.

1. **GitHub PAT:** Create a **classic** GitHub Personal Access Token (PAT) with the `public_repo` scope.
2. **Fork:** Ensure your fork of `microsoft/winget-pkgs` remains active in your GitHub account (`FittyAr`).
3. **Repository Secret:** In your **Pairee** GitHub repository settings:
   - Go to **Settings > Secrets and variables > Actions**.
   - Create a new repository secret named `WINGET_TOKEN`.
   - Paste your classic PAT as the value.

Whenever you bump the version and publish a release from draft to public on GitHub, the `.github/workflows/winget.yml` workflow will automatically run and submit a Pull Request to update the package to the new version.
