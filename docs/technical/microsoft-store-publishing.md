# Microsoft Store Publishing Guide — Pairee

This guide explains how to package and publish **Pairee** to the Microsoft Store.

---

## The Key to Publishing Without a Paid Certificate: MSIX

If you attempt to publish Pairee as a traditional `.exe` (Inno Setup) or `.msi` installer, Microsoft **requires** you to digitally sign it yourself using a paid Code Signing certificate from a trusted Certificate Authority (CA) (which costs hundreds of dollars per year).

**The Solution:** Publish Pairee as an **MSIX package**.
* When you submit an `.msix` package, the Microsoft Store **automatically signs the package on your behalf** with a trusted Microsoft certificate during the ingestion process.
* You do not need to buy or manage any certificate for store distribution.
* The installed app will be completely trusted by all Windows users without any SmartScreen warnings.

---

## Step 1 — Prerequisites

1. **Partner Center Account:** Register a developer account at the [Microsoft Partner Center](https://partner.microsoft.com/dashboard) (requires a one-time fee of ~$19 USD for individuals or ~$99 USD for companies).
2. **Reserve App Name:** 
   * Navigate to Partner Center → Apps and games → **Create a new app**.
   * Enter **Pairee** and reserve the name.
3. **Retrieve Product Identity:**
   * Go to **Product management** → **Product Identity**.
   * Copy the following values (you will need them for your manifest):
     * **Package/Identity/Name** (`30176FittyDev.Pairee`)
     * **Package/Identity/Publisher** (`CN=EDC5BDED-A726-42CD-B98E-5657B88D9832`)
     * **Publisher display name** (`FittyAr`)

---

## Step 2 — MSIX Package Structure

To package Pairee, you must create a directory (e.g., `target/msix_staging/`) containing the executable, resource folders, visual assets, and the package manifest:

```text
target/msix_staging/
├── pairee.exe              (Release binary compiled for Windows)
├── lang/                   (Translations folder)
├── help/                   (Help documents)
├── docs/                   (Technical documentation)
├── keymaps/                (Keybinding configuration presets)
├── Assets/                 (Visual PNG assets)
│   ├── Square150x150Logo.png
│   ├── Square44x44Logo.png
│   ├── StoreLogo.png
│   └── SplashScreen.png
└── AppxManifest.xml        (The package manifest file)
```

### Visual Assets Guidelines
All assets must be transparent PNGs:
* **Square150x150Logo.png:** The primary app tile logo (150x150 px).
* **Square44x44Logo.png:** App list / taskbar icon (44x44 px).
* **StoreLogo.png:** Icon displayed in the Microsoft Store listing (50x50 px).
* **SplashScreen.png:** Image shown when the package starts up (620x300 px).

---

## Step 3 — The `AppxManifest.xml` File

The package manifest is [`manifests/msix/AppxManifest.xml`](../../manifests/msix/AppxManifest.xml);
`.github/scripts/package-msix.ps1` copies it into the staging folder and
fills in the identity, the version (`X.Y.Z.0`) and the architecture. It
declares:

- the `pairee` command (`uap5:AppExecutionAlias`, console subsystem), so the
  Store installation starts from any terminal like the other installers;
- the languages of `lang/` in the variants that get their own Store listing
  (`<Resources>`: en-US, en-GB, en-AU, en-CA, en-IN, es-ES, es-MX, es-AR,
  es-CO, es-CL, es-US);
- `runFullTrust`, required by a desktop (Win32) file manager.

The listing text for those languages is in [`store-listing.md`](store-listing.md).

> [!IMPORTANT]
> The template manifest contains placeholders (`TEMPLATE_PACKAGE_IDENTITY_NAME`, `TEMPLATE_PACKAGE_IDENTITY_PUBLISHER`, and `TEMPLATE_PUBLISHER_DISPLAY_NAME`) to prevent hardcoding sensitive credentials in the source code.
> 
> - **Local Development:** `cargo xtask dev msix-package` (then `msix-cert`, `msix-sign`, `msix-install`) packages the local release build with `.github/scripts/package-msix.ps1`, the script the Release workflow uses; unset `STORE_*` variables fall back to the Pairee Store identity defined in that script.
> - **CI/CD Pipeline (GitHub Actions):** the Release workflow reads `STORE_PACKAGE_NAME`, `STORE_PUBLISHER` and `STORE_PUBLISHER_DISPLAY_NAME` (repository secrets or variables) and attaches `pairee-X.Y.Z-x64.msix` and `pairee-X.Y.Z-arm64.msix` to every release.

Installed from the Store, Pairee detects the `WindowsApps` folder and leaves
updates to the Store (`InstallMethod::MsStore`) instead of replacing itself.

## Step 4 — Compiling the MSIX Package

Use the **`MakeAppx.exe`** tool (included in the Windows SDK) to pack your staging directory into an MSIX file.

1. Open the **Developer Command Prompt for VS 2022** (so all SDK binaries are automatically in your PATH).
2. Navigate to your project directory.
3. Run the pack command:
   ```cmd
   MakeAppx.exe pack /d target\msix_staging /p target\pairee_0.6.1_x64.msix
   ```

---

## Step 5 — Local Testing (Optional but Recommended)

Windows will not allow you to install a locally compiled MSIX package unless it is digitally signed. For local testing, you can sign it with a **free self-signed certificate**:

### 1. Create a local self-signed certificate
Run PowerShell as Administrator and execute:
```powershell
# Publisher name must match the CN specified in your AppxManifest.xml
$cert = New-SelfSignedCertificate -Type Custom -Subject "CN=EDC5BDED-A726-42CD-B98E-5657B88D9832" `
   -KeyUsage DigitalSignature -FriendlyName "Pairee Local Test" `
   -CertStoreLocation "Cert:\CurrentUser\My" -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3")
```

### 2. Export the certificate and trust it
```powershell
# Export the certificate file
Export-Certificate -Cert $cert -FilePath target\pairee_test.cer

# Install it to your machine's Trusted Root folder to allow installation
Import-Certificate -FilePath target\pairee_test.cer `
   -CertStoreLocation "Cert:\LocalMachine\Root"
```

### 3. Sign the package
Open the Developer Command Prompt as Administrator and run `SignTool.exe`:
```cmd
SignTool.exe sign /fd SHA256 /a /f target\pairee_test.cer target\pairee_0.6.1_x64.msix
```
Now you can double-click `pairee_0.6.1_x64.msix` to install it locally and test its behavior!

---

## Step 6 — Submitting to the Microsoft Store

Once you have validated the package locally, you are ready to upload the **unsigned** (or self-signed) `.msix` package to Partner Center:

### Option A — Direct Partner Center Upload
1. Log in to [Microsoft Partner Center](https://partner.microsoft.com/dashboard).
2. Select your app **Pairee** and click **Start new submission**.
3. Under **Packages**, upload the `target/pairee_0.6.1_x64.msix` file.
4. Fill in the store listings, upload screenshots, set age ratings, and select pricing (Free).
5. Click **Submit to the Store**.

### Option B — Using the Microsoft Store Developer CLI
The Release workflow does this for every release (see [Every release](#every-release)); by hand, with the `msstore` CLI:
```powershell
# 1. Sign in to Partner Center with the Entra ID application (see One-time setup)
msstore reconfigure --tenantId <tenant> --sellerId <seller> --clientId <client> --clientSecret <secret>

# 2. Upload and submit the package
msstore publish target\pairee_0.6.1_x64.msix --appId 9PBSDNBTP2JZ
```

During ingestion, the Microsoft Store will verify the identity matches your account, discard the temporary signature, and **re-sign the app with the official Microsoft Store root-trusted certificate**.

---

## Every release

The Release workflow submits every published release to the Store by
itself: after the `release` environment is approved and the release is
public, its `msstore` job runs [`.github/workflows/msstore.yml`](../../.github/workflows/msstore.yml)
(next to the WinGet job), which:

1. downloads `pairee-X.Y.Z-x64.msix` and `pairee-X.Y.Z-arm64.msix` from the
   release and bundles them into `pairee-X.Y.Z.msixbundle` (`msstore publish`
   uploads one file);
2. installs the Microsoft Store Developer CLI (`microsoft/microsoft-store-apppublisher`),
   signs in with the `MSSTORE_*` secrets and uploads the bundle to a new draft
   submission of product `9PBSDNBTP2JZ`, made from the last published one
   (listings, screenshots, pricing and age ratings carry over);
3. marks the packages of the previous submission for deletion and sets
   "What's new" of every English listing from `### What's new in X.Y.Z` and of
   every Spanish one from `### Novedades de la X.Y.Z` in
   [`store-listing.md`](store-listing.md);
4. commits the submission. Certification usually takes one to three days;
   installed copies then update through the Store.

The script is [`.github/scripts/msstore-publish.ps1`](../../.github/scripts/msstore-publish.ps1).
Before tagging, add both "What's new" sections for the new version to
`store-listing.md` (at most 1,500 characters each, written from
`docs/CHANGELOG.md`): without them the job fails before uploading anything.
Without the secrets it is skipped with a warning and the release is not
affected.

To retry a submission, or to submit an earlier release: Actions → **Publish
to the Microsoft Store** → **Run workflow** with the tag (`vX.Y.Z`). A retry
replaces the pending draft. Partner Center still shows the submission under
Pairee → **Submissions**; changes to screenshots, descriptions or features are
made there by hand (and copied to `store-listing.md`).

## One-time setup

The workflow signs in to Partner Center as a Microsoft Entra ID application.

1. **Register the application.** [Azure portal](https://portal.azure.com) →
   **Microsoft Entra ID** → **App registrations** → **New registration**: name
   it (for example `Pairee Store publishing`), single tenant, no redirect URI.
   Note the **Application (client) ID** and the **Directory (tenant) ID**.
2. **Create a client secret.** In the application: **Certificates & secrets** →
   **New client secret**. Copy the **Value** at once (it is shown only once) and
   note its expiry date: renew it and the repository secret before then.
3. **Give it access to Partner Center.** [Partner Center](https://partner.microsoft.com/dashboard)
   → ⚙ **Account settings** → **User management** → **Microsoft Entra
   applications** → **Add Microsoft Entra application**: pick the application
   and give it the **Manager** role. (If the tab asks to associate the
   tenant first, do it in **Account settings** → **Tenants**.)
4. **Find the seller ID.** Partner Center → ⚙ **Account settings** → **Legal
   info** → **Developer** tab → **Seller ID**.
5. **Add the repository secrets.** GitHub → Settings → Secrets and variables →
   Actions → **New repository secret**:

   | Secret | Value |
   |---|---|
   | `MSSTORE_TENANT_ID` | Directory (tenant) ID |
   | `MSSTORE_SELLER_ID` | Seller ID |
   | `MSSTORE_CLIENT_ID` | Application (client) ID |
   | `MSSTORE_CLIENT_SECRET` | Client secret value |

To check the setup without waiting for a release, run the workflow by hand
with the tag of the current Store version: it submits that version again.
