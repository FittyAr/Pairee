# Builds dist/pairee-<Version>-<Arch>.msix (Microsoft Store / sideloading)
# from the release build of one Windows target. Called by package.sh.
#
# The package identity comes from STORE_PACKAGE_NAME, STORE_PUBLISHER and
# STORE_PUBLISHER_DISPLAY_NAME (set by release.yml).
param(
    [Parameter(Mandatory = $true)] [string] $Target,
    [Parameter(Mandatory = $true)] [ValidateSet('x64', 'arm64')] [string] $Arch,
    [Parameter(Mandatory = $true)] [string] $Version
)
$ErrorActionPreference = 'Stop'

foreach ($name in 'STORE_PACKAGE_NAME', 'STORE_PUBLISHER', 'STORE_PUBLISHER_DISPLAY_NAME') {
    if (-not [Environment]::GetEnvironmentVariable($name)) {
        throw "$name is not set"
    }
}

$staging = "target/msix_staging_$Target"
New-Item -ItemType Directory -Force -Path $staging | Out-Null
Copy-Item -Path "target/$Target/release/pairee.exe" -Destination $staging
Copy-Item -Path 'lang', 'help', 'manifests/msix/Assets' -Destination $staging -Recurse
Copy-Item -Path 'manifests/msix/AppxManifest.xml' -Destination $staging

$manifestPath = (Resolve-Path "$staging/AppxManifest.xml").Path
$manifest = [System.IO.File]::ReadAllText($manifestPath)
$manifest = $manifest.Replace('TEMPLATE_PACKAGE_IDENTITY_NAME', $env:STORE_PACKAGE_NAME)
$manifest = $manifest.Replace('TEMPLATE_PACKAGE_IDENTITY_PUBLISHER', $env:STORE_PUBLISHER)
$manifest = $manifest.Replace('TEMPLATE_PUBLISHER_DISPLAY_NAME', $env:STORE_PUBLISHER_DISPLAY_NAME)
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
& $makeappx pack /o /d $staging /p "dist/pairee-$Version-$Arch.msix"
if ($LASTEXITCODE -ne 0) {
    throw "makeappx failed with exit code $LASTEXITCODE"
}
