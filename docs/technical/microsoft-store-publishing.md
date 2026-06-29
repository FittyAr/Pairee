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

Create a file named `AppxManifest.xml` in your staging folder. Populate it with your Partner Center product identity details:

```xml
<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">

  <Identity
    Name="TEMPLATE_PACKAGE_IDENTITY_NAME"
    Publisher="TEMPLATE_PACKAGE_IDENTITY_PUBLISHER"
    Version="0.6.1.0" 
    ProcessorArchitecture="x64" />

  <Properties>
    <DisplayName>Pairee</DisplayName>
    <PublisherDisplayName>TEMPLATE_PUBLISHER_DISPLAY_NAME</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22621.0" />
  </Dependencies>

  <Resources>
    <Resource Language="en-US" />
    <Resource Language="es-ES" />
  </Resources>

  <Applications>
    <Application Id="Pairee"
      Executable="pairee.exe"
      EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="Pairee"
        Description="A modern, sleek terminal dual-panel file manager."
        BackgroundColor="transparent"
        Square150x150Logo="Assets\Square150x150Logo.png"
        Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:SplashScreen Image="Assets\SplashScreen.png" />
      </uap:VisualElements>
    </Application>
  </Applications>

  <Capabilities>
    <!-- Allows the Win32 application to run with full user access (required for file managers) -->
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
```

> [!IMPORTANT]
> The template manifest contains placeholders (`TEMPLATE_PACKAGE_IDENTITY_NAME`, `TEMPLATE_PACKAGE_IDENTITY_PUBLISHER`, and `TEMPLATE_PUBLISHER_DISPLAY_NAME`) to prevent hardcoding sensitive credentials in the source code.
> 
> - **Local Development:** The local scripts (`run.bat` and `run.sh`) automatically replace these placeholders with developer testing defaults during local packaging.
> - **CI/CD Pipeline (GitHub Actions):** To compile a production-ready package with your actual credentials, you should configure GitHub Repository secrets or variables.
> 
> ### How to configure Store variables:
> 1. Go to your GitHub repository **Settings $\rightarrow$ Secrets and variables $\rightarrow$ Actions**.
> 2. Create the following Secrets or Variables (both are supported, but Variables are recommended for public values):
>    - `STORE_PACKAGE_NAME`: The Package Identity Name (e.g. `30176FittyDev.Pairee`).
>    - `STORE_PUBLISHER`: The Package Identity Publisher CN (e.g. `CN=EDC5BDED-A726-42CD-B98E-5657B88D9832`).
>    - `STORE_PUBLISHER_DISPLAY_NAME`: The Publisher Display Name (e.g. `FittyAr`).
> 
> The pipeline dynamically injects these environment variables and substitutes them during build time. If you do not configure these secrets, the pipeline automatically falls back to our default test values.

---

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

## Automatic Updates
Once users install the MSIX version, Windows Update will automatically handle application updates. Whenever you publish a new version of the MSIX package to Partner Center:
1. Increment the version number in `<Identity Version="0.6.1.0" ... />` inside `AppxManifest.xml`.
2. Pack and upload the new `.msix` package.
3. Windows Store will push the update to all active users silently in the background.
