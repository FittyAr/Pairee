# Packages a Windows build of Pairee as an MSIX (Microsoft Store /
# sideloading). Called by package.sh for the release builds and by
# `cargo xtask dev msix-package` for a local test package.
#
#   -Exe      the pairee.exe to package
#   -Arch     x64 or arm64
#   -Version  X.Y.Z (the package version becomes X.Y.Z.0)
#   -OutFile  the .msix to write
#
# The package identity comes from STORE_PACKAGE_NAME, STORE_PUBLISHER and
# STORE_PUBLISHER_DISPLAY_NAME (repository secrets or variables in the
# Release workflow); unset, the Pairee Store identity below is used.
param(
    [Parameter(Mandatory = $true)] [string] $Exe,
    [Parameter(Mandatory = $true)] [ValidateSet('x64', 'arm64')] [string] $Arch,
    [Parameter(Mandatory = $true)] [string] $Version,
    [Parameter(Mandatory = $true)] [string] $OutFile
)
$ErrorActionPreference = 'Stop'

function Get-Identity([string] $name, [string] $default) {
    $value = [Environment]::GetEnvironmentVariable($name)
    if ($value) { return $value }
    return $default
}
$packageName = Get-Identity 'STORE_PACKAGE_NAME' '30176FittyDev.Pairee'
$publisher = Get-Identity 'STORE_PUBLISHER' 'CN=EDC5BDED-A726-42CD-B98E-5657B88D9832'
$publisherDisplayName = Get-Identity 'STORE_PUBLISHER_DISPLAY_NAME' 'FittyAr'

# Staged under target/, never next to the output (dist/ is uploaded whole).
$staging = Join-Path 'target' ('msix_staging_' + [System.IO.Path]::GetFileNameWithoutExtension($OutFile))
if (Test-Path $staging) {
    Remove-Item -Recurse -Force $staging
}
New-Item -ItemType Directory -Force -Path $staging | Out-Null
Copy-Item -Path $Exe -Destination (Join-Path $staging 'pairee.exe')
Copy-Item -Path 'lang', 'help', 'manifests/msix/Assets' -Destination $staging -Recurse
Copy-Item -Path 'manifests/msix/AppxManifest.xml' -Destination $staging

$manifestPath = (Resolve-Path "$staging/AppxManifest.xml").Path
$manifest = [System.IO.File]::ReadAllText($manifestPath)
$manifest = $manifest.Replace('TEMPLATE_PACKAGE_IDENTITY_NAME', $packageName)
$manifest = $manifest.Replace('TEMPLATE_PACKAGE_IDENTITY_PUBLISHER', $publisher)
$manifest = $manifest.Replace('TEMPLATE_PUBLISHER_DISPLAY_NAME', $publisherDisplayName)
# \b: the package Version only, not TargetDeviceFamily's MinVersion.
$manifest = $manifest -creplace '\bVersion="[^"]+"', "Version=`"$Version.0`""
$manifest = $manifest -creplace '\bProcessorArchitecture="[^"]+"', "ProcessorArchitecture=`"$Arch`""
[System.IO.File]::WriteAllText($manifestPath, $manifest, (New-Object System.Text.UTF8Encoding($false)))

$makeappx = Get-ChildItem -Path 'C:\Program Files (x86)\Windows Kits\10\bin' -Filter 'makeappx.exe' -Recurse |
    Where-Object { $_.FullName -match '\\x64\\' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $makeappx) {
    $makeappx = 'makeappx.exe'
}
& $makeappx pack /o /d $staging /p $OutFile
if ($LASTEXITCODE -ne 0) {
    throw "makeappx failed with exit code $LASTEXITCODE"
}
