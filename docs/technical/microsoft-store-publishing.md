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
You can also automate the submission using the `msstore` CLI tool:
```powershell
# 1. Initialize configuration (logs you in to Partner Center via Entra ID application)
msstore init

# 2. Upload and submit the package
msstore publish target\pairee_0.6.1_x64.msix
```

During ingestion, the Microsoft Store will verify the identity matches your account, discard the temporary signature, and **re-sign the app with the official Microsoft Store root-trusted certificate**.

---

## Every release

The Release workflow builds the MSIX packages but does **not** submit them:
the Store keeps the last version uploaded to Partner Center (it stayed on
0.6.1 from July to October 2026 because of this). After each release:

1. Download `pairee-X.Y.Z-x64.msix` and `pairee-X.Y.Z-arm64.msix` from the
   GitHub release (the version inside is already `X.Y.Z.0`).
2. Partner Center → Pairee → **Start update** → **Packages**: upload both and
   remove the previous ones.
3. **Store listings**: paste "What's new" from [`store-listing.md`](store-listing.md)
   into every language (update that file first from `docs/CHANGELOG.md`).
4. **Submit to the Store**. Certification usually takes one to three days;
   installed copies then update through the Store.
