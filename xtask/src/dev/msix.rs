//! MSIX developer actions (Windows only): package the release build with the
//! script the Release workflow uses, then sign it with a self-signed test
//! certificate and install it.

use std::path::Path;
use std::process::Command;

use crate::release::version;
use crate::{Result, cmd};

/// The packaging script shared with the Release workflow.
const SCRIPT: &str = ".github/scripts/package-msix.ps1";
const RELEASE_EXE: &str = "target/release/pairee.exe";
const TEST_CERT: &str = "target/pairee_test.cer";

/// Sets `$publisher` to the package publisher recorded in `$env:PAIREE_MSIX`
/// (the certificate subject must match it).
const READ_PUBLISHER: &str = "\
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path $env:PAIREE_MSIX))
try {
    $reader = New-Object IO.StreamReader($zip.GetEntry('AppxManifest.xml').Open())
    $manifest = [xml]$reader.ReadToEnd()
    $reader.Dispose()
} finally { $zip.Dispose() }
$publisher = $manifest.Package.Identity.Publisher
";

/// Release architecture of this machine.
fn arch() -> &'static str {
    if std::env::consts::ARCH == "aarch64" {
        "arm64"
    } else {
        "x64"
    }
}

fn local_msix() -> String {
    format!("target/pairee_local_{}.msix", arch())
}

fn windows_only() -> Result {
    if cfg!(windows) {
        Ok(())
    } else {
        Err("MSIX packaging is Windows-only".into())
    }
}

/// Runs `script` in Windows PowerShell with the package and certificate
/// paths in `PAIREE_MSIX` and `PAIREE_CERT`.
fn powershell(script: &str) -> Result {
    let script = format!("$ErrorActionPreference = 'Stop'\n{script}");
    let ok = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .env("PAIREE_MSIX", local_msix())
        .env("PAIREE_CERT", TEST_CERT)
        .status()
        .map_err(|e| format!("cannot start powershell: {e}"))?
        .success();
    if ok {
        Ok(())
    } else {
        Err("the PowerShell step failed".into())
    }
}

fn require(path: &str, hint: &str) -> Result {
    if Path::new(path).is_file() {
        Ok(())
    } else {
        Err(format!("{path} not found: {hint}").into())
    }
}

/// Builds the release binary and packages it as `target/pairee_local_<arch>.msix`.
pub fn package() -> Result {
    windows_only()?;
    cmd::run("cargo", &["build", "--release"])?;
    let version = version::cargo_version()?.to_string();
    let out = local_msix();
    cmd::run(
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            SCRIPT,
            "-Exe",
            RELEASE_EXE,
            "-Arch",
            arch(),
            "-Version",
            &version,
            "-OutFile",
            &out,
        ],
    )
}

/// Creates a code-signing certificate for the package publisher in the user
/// store and trusts it machine-wide (TrustedPeople; needs an elevated shell).
pub fn create_certificate() -> Result {
    windows_only()?;
    require(&local_msix(), "run `cargo xtask dev msix-package` first")?;
    powershell(&format!(
        "{READ_PUBLISHER}
$cert = New-SelfSignedCertificate -Type Custom -Subject $publisher -KeyUsage DigitalSignature `
    -FriendlyName 'Pairee Local Test' -CertStoreLocation 'Cert:\\CurrentUser\\My' `
    -TextExtension @('2.5.29.37={{text}}1.3.6.1.5.5.7.3.3', '2.5.29.19={{text}}')
Export-Certificate -Cert $cert -FilePath $env:PAIREE_CERT | Out-Null
Import-Certificate -FilePath $env:PAIREE_CERT -CertStoreLocation 'Cert:\\LocalMachine\\TrustedPeople' | Out-Null
Write-Host \"Certificate $publisher saved to $env:PAIREE_CERT and trusted.\""
    ))
}

/// Signs the local package with the newest test certificate of its publisher.
pub fn sign() -> Result {
    windows_only()?;
    require(&local_msix(), "run `cargo xtask dev msix-package` first")?;
    require(TEST_CERT, "run `cargo xtask dev msix-cert` first")?;
    powershell(&format!(
        "{READ_PUBLISHER}
$cert = Get-ChildItem 'Cert:\\CurrentUser\\My' | Where-Object Subject -eq $publisher |
    Sort-Object NotAfter -Descending | Select-Object -First 1
if (-not $cert) {{ throw \"no certificate for $publisher in Cert:\\CurrentUser\\My\" }}
$signtool = Get-ChildItem 'C:\\Program Files (x86)\\Windows Kits\\10\\bin' -Filter signtool.exe -Recurse |
    Where-Object FullName -match '\\\\x64\\\\' | Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $signtool) {{ $signtool = 'signtool.exe' }}
& $signtool sign /fd SHA256 /sha1 $cert.Thumbprint $env:PAIREE_MSIX
exit $LASTEXITCODE"
    ))
}

/// Installs the signed local package for the current user.
pub fn install() -> Result {
    windows_only()?;
    require(&local_msix(), "run `cargo xtask dev msix-package` first")?;
    powershell("Add-AppxPackage -Path $env:PAIREE_MSIX\nWrite-Host 'Package installed.'")
}
