# bump_version.ps1
# Bumps the version in Cargo.toml, commits, tags, and pushes to trigger GitHub CI/CD releases.

$ErrorActionPreference = "Stop"

# Ensure we are in the project root
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ($scriptDir) {
    Set-Location $scriptDir
    Set-Location ..
}

if (-not (Test-Path "Cargo.toml")) {
    Write-Error "Cargo.toml not found. Make sure this script is in the 'scripts' directory of the project."
    exit 1
}

# Resolve current branch
$branch = git branch --show-current
if ([string]::IsNullOrWhiteSpace($branch)) {
    $branch = "main"
}

# Pre-flight authentication and write permission check
Write-Host "Checking Git authentication and push permissions for origin ($branch)..." -ForegroundColor Yellow
$env:GIT_TERMINAL_PROMPT = "0"
$oldEAP = $ErrorActionPreference
$ErrorActionPreference = "Continue"
git push --dry-run origin $branch
$exitCode = $LASTEXITCODE
$ErrorActionPreference = $oldEAP
Remove-Item env:GIT_TERMINAL_PROMPT

if ($exitCode -ne 0) {
    Write-Error "Git authentication failed or you do not have push permissions for origin."
    Write-Host "Please ensure you are logged into GitHub (e.g. via GitHub Desktop or 'gh auth login') and that your credential helper is active." -ForegroundColor Yellow
    Write-Host "To test manually, run: git push --dry-run origin $branch" -ForegroundColor Yellow
    exit 1
}
Write-Host "Git authentication successful." -ForegroundColor Green

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "       Pairee Version Bump & Release      " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Check if git has uncommitted changes
$gitStatus = git status --porcelain
if ($gitStatus) {
    Write-Host "[WARNING] You have uncommitted changes in your repository:" -ForegroundColor Yellow
    Write-Host $gitStatus
    $choice = Read-Host "Do you want to proceed anyway? (y/n)"
    if ($choice -ne 'y' -and $choice -ne 'Y') {
        Write-Host "Aborted."
        exit 0
    }
}

# 2. Get current version from Cargo.toml
$cargoToml = Get-Content -Raw -Path Cargo.toml
if ($cargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
    $currentVersion = $Matches[1]
    Write-Host "Current version in Cargo.toml: $currentVersion" -ForegroundColor Cyan
    
    # Suggest next patch version
    $parts = $currentVersion -split '\.'
    if ($parts.Count -eq 3) {
        $nextPatch = "$($parts[0]).$($parts[1]).$([int]$parts[2] + 1)"
    } else {
        $nextPatch = $currentVersion
    }
    
    # Prompt user for new version
    $newVersion = Read-Host "Enter new version [$nextPatch]"
    if ([string]::IsNullOrWhiteSpace($newVersion)) {
        $newVersion = $nextPatch
    }
    
    # Validate version format (x.y.z)
    if ($newVersion -notmatch '^\d+\.\d+\.\d+$') {
        Write-Error "Invalid version format. Must be like 0.1.0"
        exit 1
    }
    
    # 3. Update Cargo.toml and installer.iss
    Write-Host "Updating Cargo.toml to version $newVersion..." -ForegroundColor Yellow
    $newCargoToml = [regex]::Replace($cargoToml, '(?m)^version\s*=\s*"[^"]+"', "version = `"$newVersion`"")
    Set-Content -Path Cargo.toml -Value $newCargoToml
    
    if (Test-Path "installer.iss") {
        Write-Host "Updating installer.iss to version $newVersion..." -ForegroundColor Yellow
        $issContent = Get-Content -Raw -Path installer.iss
        $newIssContent = [regex]::Replace($issContent, '(?m)^#define\s+AppVersion\s+"[^"]+"', "#define AppVersion `"$newVersion`"")
        Set-Content -Path installer.iss -Value $newIssContent
    }

    # 3c. Update local Winget manifest files
    $wingetBaseDir = "manifests/f/FittyAr/Pairee"
    $currentManifestDir = Join-Path $wingetBaseDir $currentVersion
    $newManifestDir = Join-Path $wingetBaseDir $newVersion

    if (-not (Test-Path $currentManifestDir)) {
        # Try to find any version directory under manifests/f/FittyAr/Pairee
        if (Test-Path $wingetBaseDir) {
            $anyVersionDir = Get-ChildItem -Path $wingetBaseDir -Directory | Select-Object -First 1
            if ($anyVersionDir) {
                $currentManifestDir = $anyVersionDir.FullName
            }
        }
    }

    if (Test-Path $currentManifestDir) {
        if ($currentManifestDir -ne $newManifestDir) {
            Write-Host "Migrating WinGet manifests from $currentManifestDir to $newManifestDir..." -ForegroundColor Yellow
            New-Item -ItemType Directory -Path $newManifestDir -Force | Out-Null
            Copy-Item -Path "$currentManifestDir\*" -Destination $newManifestDir -Force
            Remove-Item -Path $currentManifestDir -Recurse -Force
        }

        Write-Host "Updating local WinGet manifests to version $newVersion..." -ForegroundColor Yellow
        $yamlFiles = Get-ChildItem -Path $newManifestDir -Filter "FittyAr.Pairee*.yaml"
        foreach ($file in $yamlFiles) {
            $content = Get-Content -Raw -Path $file.FullName
            # Update PackageVersion
            $content = [regex]::Replace($content, '(?m)^PackageVersion:\s*.*', "PackageVersion: $newVersion")
            # Update InstallerUrls
            $content = [regex]::Replace($content, '(?i)(InstallerUrl:\s*https://github.com/FittyAr/Pairee/releases/download/)v[^/]+(.*\.exe)', '$1v' + $newVersion + '$2')
            $content = [regex]::Replace($content, '(?i)pairee-setup-[0-9]+\.[0-9]+\.[0-9]+-(x64|arm64)\.exe', 'pairee-setup-' + $newVersion + '-$1.exe')
            # Update ReleaseNotesUrl
            $content = [regex]::Replace($content, '(?i)(ReleaseNotesUrl:\s*https://github.com/FittyAr/Pairee/releases/tag/)v.*', '$1v' + $newVersion)
            
            # Save back (without BOM)
            $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
            [System.IO.File]::WriteAllText($file.FullName, $content, $utf8NoBom)
        }
    } else {
        Write-Host "[WARNING] No existing WinGet manifest directory found to migrate." -ForegroundColor Yellow
    }

    # 3b. Stamp CHANGELOG.md: rename [Unreleased] -> [vX.Y.Z] - YYYY-MM-DD and add a fresh [Unreleased]
    $changelogPath = "CHANGELOG.md"
    if (Test-Path $changelogPath) {
        Write-Host "Stamping CHANGELOG.md for v$newVersion..." -ForegroundColor Yellow
        $today = (Get-Date -Format "yyyy-MM-dd")
        $changelogContent = Get-Content -Path $changelogPath -Encoding UTF8

        $unreleasedPattern = '^## \[Unreleased\]'
        $foundUnreleased = $false
        $newChangelog = @()

        foreach ($line in $changelogContent) {
            if (-not $foundUnreleased -and $line -match $unreleasedPattern) {
                $foundUnreleased = $true
                # Insert fresh [Unreleased] block before the versioned heading
                $newChangelog += "## [Unreleased]"
                $newChangelog += ""
                $newChangelog += "---"
                $newChangelog += ""
                # Replace this line with the version heading
                $newChangelog += "## [v$newVersion] - $today"
            } else {
                $newChangelog += $line
            }
        }

        if (-not $foundUnreleased) {
            Write-Host "[WARNING] '## [Unreleased]' section not found in CHANGELOG.md - skipping changelog stamp." -ForegroundColor Yellow
        } else {
            # Write back using UTF8 without BOM
            $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
            [System.IO.File]::WriteAllLines((Resolve-Path $changelogPath).Path, $newChangelog, $utf8NoBom)
            Write-Host "CHANGELOG.md stamped successfully." -ForegroundColor Green
        }
    } else {
        Write-Host "[WARNING] CHANGELOG.md not found - skipping changelog stamp." -ForegroundColor Yellow
    }

    # 4. Run cargo check to update Cargo.lock
    Write-Host "Running cargo check to update Cargo.lock..." -ForegroundColor Yellow
    try {
        cargo check
    } catch {
        Write-Error "Cargo check failed. Reverting Cargo.toml..."
        Set-Content -Path Cargo.toml -Value $cargoToml
        exit 1
    }
    
    # 5. Git Commit and Tag Confirmation
    $branch = git branch --show-current
    if ([string]::IsNullOrWhiteSpace($branch)) {
        $branch = "main"
    }
    
    Write-Host ""
    Write-Host "Summary of actions to perform:" -ForegroundColor Yellow
    Write-Host "  - Stage and commit changes (Cargo.toml, Cargo.lock, installer.iss, CHANGELOG.md, manifests/f/FittyAr/Pairee/*)"
    Write-Host "  - Create git tag v$newVersion"
    Write-Host "  - Push commit and tag to origin ($branch)"
    Write-Host ""
    
    $confirm = Read-Host "Are you sure you want to commit, tag, and push? (y/n)"
    if ($confirm -ne 'y' -and $confirm -ne 'Y') {
        Write-Host "Operation cancelled. Cargo.toml/Cargo.lock/installer.iss were updated but no Git changes were committed or pushed." -ForegroundColor Yellow
        exit 0
    }
    
    # Commit and tag
    Write-Host "Staging changes..." -ForegroundColor Yellow
    git add Cargo.toml Cargo.lock installer.iss
    if (Test-Path "CHANGELOG.md") {
        git add CHANGELOG.md
    }
    if (Test-Path "manifests/f/FittyAr/Pairee") {
        git add manifests/f/
    }
    git commit -m "Bump version to v$newVersion"
    
    Write-Host "Creating git tag v$newVersion..." -ForegroundColor Yellow
    git tag -a "v$newVersion" -m "Release v$newVersion"
    
    # Push to origin
    Write-Host "Pushing commits and tag to origin..." -ForegroundColor Yellow
    try {
        git push origin $branch
        git push origin "v$newVersion"
        Write-Host "Successfully bumped version to v$newVersion and pushed to GitHub!" -ForegroundColor Green
        Write-Host "GitHub Actions will now build binaries and create a draft release." -ForegroundColor Green
        Write-Host "Review the draft release on GitHub and publish it when ready." -ForegroundColor Cyan
        Write-Host ""
        Write-Host "[WinGet Notice]" -ForegroundColor Yellow
        Write-Host "Once you publish the draft release on GitHub, the automated WinGet action will run" -ForegroundColor Yellow
        Write-Host "and automatically submit the update to microsoft/winget-pkgs." -ForegroundColor Yellow
        Write-Host "NOTE: Make sure your WINGET_TOKEN secret is set in the repo." -ForegroundColor Yellow
    } catch {
        Write-Error "Failed to push to GitHub. Check your internet connection or repository permissions."
        Write-Host "Note: The commit and tag were created locally. You can push manually using:" -ForegroundColor Yellow
        Write-Host "  git push origin $branch"
        Write-Host "  git push origin v$newVersion"
    }
} else {
    Write-Error "Could not find 'version = \"...\"' in Cargo.toml"
    exit 1
}
