# extract_changelog.ps1
# Extracts a specific version section from CHANGELOG.md and outputs it to stdout.
# Usage: .\scripts\extract_changelog.ps1 v0.5.1
#        .\scripts\extract_changelog.ps1 Unreleased
#
# The output is suitable for use as a GitHub Release body.

param (
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

# Resolve paths relative to this script's parent directory (project root)
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Normalise version and determine target file
if ($Version -ieq "Unreleased") {
    $targetPath = Join-Path $scriptDir "..\docs\UNRELEASED.md"
    $sectionHeader = "## [Unreleased]"
} else {
    $targetPath = Join-Path $scriptDir "..\docs\CHANGELOG.md"
    if ($Version -match '^v') {
        $sectionHeader = "## [$Version]"
    } else {
        $sectionHeader = "## [v$Version]"
    }
}

if (-not (Test-Path $targetPath)) {
    Write-Error "File not found at: $targetPath"
    exit 1
}

$lines = Get-Content -Path $targetPath -Encoding UTF8

$inSection = $false
$sectionLines = @()

foreach ($line in $lines) {
    if ($line.TrimEnd() -eq $sectionHeader -or $line.TrimStart().StartsWith("$sectionHeader ")) {
        $inSection = $true
        continue  # Skip the header line itself; the release body starts from the content
    }

    if ($inSection) {
        # Next top-level ## heading means we've left this section
        if ($line -match '^## \[') {
            break
        }
        $sectionLines += $line
    }
}

if ($sectionLines.Count -eq 0) {
    Write-Error "Section '$sectionHeader' not found or is empty in CHANGELOG.md"
    exit 1
}

# Trim leading/trailing blank lines
$firstContent = 0
$lastContent = $sectionLines.Count - 1

while ($firstContent -le $lastContent -and [string]::IsNullOrWhiteSpace($sectionLines[$firstContent])) {
    $firstContent++
}
while ($lastContent -ge $firstContent -and [string]::IsNullOrWhiteSpace($sectionLines[$lastContent])) {
    $lastContent--
}

$trimmed = $sectionLines[$firstContent..$lastContent]
$trimmed | Write-Output
