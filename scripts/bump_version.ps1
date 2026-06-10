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
    Write-Host "  - Stage and commit changes (Cargo.toml, Cargo.lock, installer.iss)"
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
    git commit -m "Bump version to v$newVersion"
    
    Write-Host "Creating git tag v$newVersion..." -ForegroundColor Yellow
    git tag -a "v$newVersion" -m "Release v$newVersion"
    
    # Push to origin
    Write-Host "Pushing commits and tag to origin..." -ForegroundColor Yellow
    try {
        git push origin $branch
        git push origin "v$newVersion"
        Write-Host "Successfully bumped version to v$newVersion and pushed to GitHub!" -ForegroundColor Green
        Write-Host "GitHub Actions will now compile and publish the release." -ForegroundColor Green
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
