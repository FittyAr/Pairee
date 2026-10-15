# Submits a release to the Microsoft Store. Called by
# .github/workflows/msstore.yml once `msstore reconfigure` has the Partner
# Center credentials; runs from the repository root at the release tag.
#
#   -Version    X.Y.Z
#   -Dist       folder with pairee-X.Y.Z-x64.msix and pairee-X.Y.Z-arm64.msix
#   -ProductId  Store product ID
#
# `msstore publish` uploads one file, so both packages go in one
# pairee-X.Y.Z.msixbundle. It stays a draft while the packages of the last
# submission are removed and "What's new" of every English and Spanish
# listing is set from docs/technical/store-listing.md; then it is committed
# for certification.
param(
    [Parameter(Mandatory = $true)] [string] $Version,
    [Parameter(Mandatory = $true)] [string] $Dist,
    [Parameter(Mandatory = $true)] [string] $ProductId
)
$ErrorActionPreference = 'Stop'

$listingFile = 'docs/technical/store-listing.md'
# Store limit for "What's new".
$maxReleaseNotes = 1500

# The text under `heading` in store-listing.md, up to the next heading or rule.
function Get-WhatsNew([string] $heading) {
    $lines = Get-Content -LiteralPath $listingFile -Encoding utf8
    $start = [Array]::IndexOf($lines, $heading)
    if ($start -lt 0) {
        throw "$listingFile has no '$heading' section: add it before submitting $Version"
    }
    $body = foreach ($line in $lines[($start + 1)..($lines.Count - 1)]) {
        if ($line -match '^(#|---)') { break }
        $line
    }
    $text = ($body -join "`n").Trim()
    if ($text.Length -gt $maxReleaseNotes) {
        throw "'$heading' in $listingFile has $($text.Length) characters (Store limit: $maxReleaseNotes)"
    }
    return $text
}

function Invoke-MSStore {
    & msstore @args
    if ($LASTEXITCODE -ne 0) {
        throw "msstore $($args[0]) $($args[1]) failed with exit code $LASTEXITCODE"
    }
}

# Checked first: nothing is uploaded when the listing text is missing.
$whatsNew = @{
    en = Get-WhatsNew "### What's new in $Version"
    es = Get-WhatsNew "### Novedades de la $Version"
}

# ── One bundle with both architectures ──────────────────────────────────
$work = Join-Path ([IO.Path]::GetTempPath()) "msstore-$Version"
$packages = Join-Path $work 'packages'
if (Test-Path $work) {
    Remove-Item -Recurse -Force $work
}
New-Item -ItemType Directory -Force -Path $packages | Out-Null
foreach ($arch in 'x64', 'arm64') {
    Copy-Item -LiteralPath (Join-Path $Dist "pairee-$Version-$arch.msix") -Destination $packages
}
$bundleName = "pairee-$Version.msixbundle"
$bundle = Join-Path $work $bundleName
& (Join-Path $PSScriptRoot 'makeappx.ps1') bundle /o /bv "$Version.0" /d $packages /p $bundle

# A new draft from the last published submission, with the bundle uploaded
# (an earlier draft is replaced, so a retry starts clean).
Invoke-MSStore publish $bundle --appId $ProductId --noCommit

# ── Packages and "What's new" ───────────────────────────────────────────
$output = (& msstore submission get $ProductId) -join "`n"
if ($LASTEXITCODE -ne 0) {
    throw "msstore submission get failed with exit code $LASTEXITCODE"
}
# The JSON follows any status lines.
$json = $output.Substring([Math]::Max(0, $output.IndexOf("`n{") + 1))
$submission = $json | ConvertFrom-Json

# `publish` only replaces a package of the same type; the bundle supersedes
# the separate .msix packages of earlier submissions.
foreach ($package in $submission.ApplicationPackages) {
    if ($package.FileName -ne $bundleName -and $package.FileStatus -eq 'Uploaded') {
        Write-Host "Removing package $($package.FileName)"
        $package.FileStatus = 'PendingDelete'
    }
}

foreach ($listing in $submission.Listings.PSObject.Properties) {
    $language = $listing.Name.Substring(0, 2).ToLowerInvariant()
    if (-not $whatsNew.ContainsKey($language)) {
        Write-Host "::warning::Store listing $($listing.Name) has no text in $listingFile; What's new left as it was."
        continue
    }
    Write-Host "What's new: $($listing.Name)"
    $listing.Value.BaseListing.ReleaseNotes = $whatsNew[$language]
}

# A file, not an argument: 11 listings exceed the command-line limit.
$update = Join-Path $work 'submission.json'
$submission | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $update -Encoding utf8NoBOM
Invoke-MSStore submission update $ProductId $update

# ── Certification ───────────────────────────────────────────────────────
Invoke-MSStore submission publish $ProductId
Invoke-MSStore submission status $ProductId
