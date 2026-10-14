# WinGet Submission Guide — Pairee

> **Identifier:** `FittyAr.Pairee`
> **Manifest version:** `1.12.0`
> **Workflows:** [`.github/workflows/winget.yml`](../.github/workflows/winget.yml) (called by [`release.yml`](../.github/workflows/release.yml))
> **Manifest template:** [`manifests/f/FittyAr/Pairee/<version>/`](../manifests/f/FittyAr/Pairee/)

---

## How it works

`FittyAr.Pairee` is **not yet in** [`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs)
(the 0.6.0 pull requests #394315 and #394525 were closed unmerged; the first
one failed on a missing MSVC runtime). `winget.yml` handles both cases on its
own, after every published release:

| Package in winget-pkgs? | What `winget.yml` does |
|---|---|
| **No** (first submission) | Checks out the release tag, writes the manifests of `manifests/f/FittyAr/Pairee/<version>/` with the installer URLs and SHA-256 of the release (`cargo xtask winget-manifests <tag> <dist> <out>`, reading the published `pairee-setup-*.exe.sha256`), downloads Microsoft's `wingetcreate.exe` (version and SHA-256 pinned in the workflow) and runs `wingetcreate submit`, which opens a **new-package** pull request. |
| **Yes** | Opens an **update** pull request with [`winget-releaser`](https://github.com/vedantmgoyal9/winget-releaser), which starts from the manifests already in winget-pkgs and swaps in the new installers (`installers-regex: '^pairee-setup-[0-9.]+-(x64|arm64)\.exe$'`). |

Both open the pull request from the `WINGET_TOKEN` owner's fork of
`microsoft/winget-pkgs` (the tools create the fork if it is missing). The
workflow can be rerun by hand: **Actions → Publish to WinGet → Run workflow**
with the tag.

### The manifests

- `FittyAr.Pairee.yaml` (version), `FittyAr.Pairee.locale.en-US.yaml`
  (default locale), `FittyAr.Pairee.locale.es-ES.yaml`,
  `FittyAr.Pairee.installer.yaml`.
- Installer: `InstallerType: inno`, `Scope: user` (the installer is per-user
  since REL-01: `PrivilegesRequired=lowest`, `{localappdata}\Programs\pairee`),
  `ProductCode: '{D37E8417-C08D-43EC-4FE5-87673A12B57F}_is1'` (the Inno
  `AppId` plus `_is1`), `UpgradeBehavior: install`, `Commands: [pairee]`,
  x64 and arm64 installers.
- **No dependencies**: the MSVC builds link the C runtime statically
  (`+crt-static` in `.cargo/config.toml`); `pairee.exe` imports no
  `vcruntime140.dll` / `api-ms-win-crt-*` (checked on the 0.9.0 binary), so
  the VC++ redistributable that sank the 0.6.0 submission is not needed.
- `cargo xtask release` moves the folder to the new version and updates
  `PackageVersion`, the URLs and `ReleaseDate`. The `InstallerSha256` values
  in the repository are a placeholder (zeros): the hashes exist only once the
  Release workflow has built the installers, and `winget-manifests` fills
  them in.

Validate a generated set locally (Windows):

```powershell
gh release download v0.11.0 --pattern 'pairee-setup-*.exe.sha256' --dir $env:TEMP\pairee-dist
cargo xtask winget-manifests v0.11.0 $env:TEMP\pairee-dist $env:TEMP\pairee-winget
winget validate --manifest $env:TEMP\pairee-winget
```

---

## One-time setup (maintainer)

1. **Token.** GitHub → Settings → Developer settings → Personal access tokens
   → **Tokens (classic)** → *Generate new token (classic)*:
   - scope **`public_repo`** (nothing else);
   - expiration: up to 1 year — note the date, the workflow fails once it
     expires.

   Fine-grained tokens do not work: the pull request goes to a repository the
   token owner does not own.
2. **Secret.** `https://github.com/FittyAr/Pairee/settings/secrets/actions`
   → *New repository secret* → name `WINGET_TOKEN`, value: the token.
3. **Fork.** Fork `microsoft/winget-pkgs` into the token owner's account
   (`FittyAr/winget-pkgs`) — the tools create it otherwise, but an existing
   fork avoids a race on the first run. Keep it; do not delete it while
   pull requests are open. Close or delete the old branches of the closed
   0.6.0 pull requests if they are still there.
4. **CLA.** On the first pull request, the `microsoft-github-policy-service`
   bot asks for the Contributor License Agreement. Reply on the pull request
   with exactly:

   ```text
   @microsoft-github-policy-service agree
   ```

   Nothing is merged before that.

---

## First release with the new flow

1. Release as usual: `cargo xtask release` → the tag starts `release.yml` →
   approve the `release` environment → the release is published.
2. `release.yml` calls `winget.yml`; the job log says *"FittyAr.Pairee is not
   in winget-pkgs yet: submitting it as a new package"* and prints the four
   manifests. `wingetcreate` prints the pull request URL.
3. On the pull request: sign the CLA (above) and watch the checks. A
   first-time package also waits for a human moderator (hours to days).
   Answer review comments by pushing to the pull request branch of the fork,
   or close it and rerun the workflow.
4. Once merged (`winget install FittyAr.Pairee` may take a few hours to see
   it), later releases take the update path by themselves.

If a release is published while the first pull request is still open, the
workflow opens a second new-package pull request for the newer version;
close the older one.

---

## Developer shortcuts

`cargo xtask dev` (replaces `run.bat` / `run.sh`) has the WinGet entries:
`winget-install`, `winget-install-x64`, `winget-install-arm64`,
`winget-upgrade`, `winget-uninstall` (`cargo xtask dev <entry>` runs one
directly; Windows only).

---

## Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `Repository secret WINGET_TOKEN is not set` | Secret missing. | One-time setup, steps 1–2. |
| `wingetcreate submit failed` / `Bad credentials` | Token expired or without `public_repo`. | New classic token, update the secret, rerun the workflow. |
| `wingetcreate.exe does not match its pinned SHA-256` | The download changed. | Check the release on github.com/microsoft/winget-create; update `WINGETCREATE_VERSION` and `WINGETCREATE_SHA256` together (SHA-256 from `wingetcreate.exe.txt`). |
| `no WinGet manifests for X.Y.Z` | The tag has no `manifests/f/FittyAr/Pairee/X.Y.Z/`. | Releases made with `cargo xtask release` move the folder; otherwise create it from the previous one. |
| `Needs-CLA` label | CLA not signed. | Reply `@microsoft-github-policy-service agree`. |
| `Error-Hash-Mismatch` | The installer changed after the `.sha256` was published. | Never replace release assets; publish a new version. |
| `Manifest-Validation-Error` | Schema problem. | `winget validate --manifest <folder>` locally (see above). |
| `winget install` finds nothing after the merge | Index not refreshed yet. | Wait a few hours. |
