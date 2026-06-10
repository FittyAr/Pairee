# NCRust Windows PowerShell Installer
# Installs NCRust executable and registers assets into %APPDATA%.

$ErrorActionPreference = "Stop"

$repo = "FittyAr/NCRust"
$installDir = Join-Path $HOME "AppData\Local\Programs\ncrust"
$configDir = Join-Path $env:APPDATA "ncrust\config"

Write-Host "NCRust Installer for Windows" -ForegroundColor Blue
Write-Host "=============================="

# 1. Fetch Version
Write-Host "Fetching latest version info..."
$releasesUrl = "https://api.github.com/repos/$repo/releases/latest"
try {
    # Using UseBasicParsing to avoid dependency on Internet Explorer engine
    $release = Invoke-RestMethod -Uri $releasesUrl -UseBasicParsing
    $version = $release.tag_name
} catch {
    Write-Error "Failed to retrieve latest release version from GitHub API: $_"
    exit 1
}

Write-Host "Latest version found: $version" -ForegroundColor Green

# 2. Setup folders
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
}
if (-not (Test-Path (Join-Path $configDir "lang"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $configDir "lang") | Out-Null
}
if (-not (Test-Path (Join-Path $configDir "help"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $configDir "help") | Out-Null
}

# 3. Download and Extract ZIP
$tempDir = Join-Path $env:TEMP "ncrust_install_$(Get-Date -Format 'yyyyMMddHHmmss')"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

$zipName = "ncrust-$version-x86_64-pc-windows-msvc.zip"
$downloadUrl = "https://github.com/$repo/releases/download/$version/$zipName"
$zipPath = Join-Path $tempDir $zipName

Write-Host "Downloading $zipName..."
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing
} catch {
    Write-Error "Failed to download release file: $_"
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
    exit 1
}

Write-Host "Extracting archive..."
try {
    Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
} catch {
    Write-Error "Failed to extract ZIP file: $_"
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
    exit 1
}

# 4. Copying Files
Write-Host "Installing files..."
$extractedFolder = Join-Path $tempDir "ncrust-$version-x86_64-pc-windows-msvc"

# Copy executable
Copy-Item -Path (Join-Path $extractedFolder "ncrust.exe") -Destination $installDir -Force

# Copy resources
Copy-Item -Path (Join-Path $extractedFolder "lang\*") -Destination (Join-Path $configDir "lang") -Force -Recurse
Copy-Item -Path (Join-Path $extractedFolder "help\*") -Destination (Join-Path $configDir "help") -Force -Recurse

# Clean up temp
Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

Write-Host "==============================" -ForegroundColor Blue
Write-Host "NCRust version $version installed successfully!" -ForegroundColor Green
Write-Host "Executable location: $installDir\ncrust.exe" -ForegroundColor Blue
Write-Host "Configuration and resources folder: $configDir" -ForegroundColor Blue
Write-Host ""

# 5. PATH setup
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -split ';' -notcontains $installDir) {
    Write-Host "Adding NCRust directory to User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
    # Update current process PATH
    $env:PATH = "$env:PATH;$installDir"
    Write-Host "PATH updated successfully. You might need to restart your terminal/IDE to refresh PATH changes." -ForegroundColor Green
} else {
    Write-Host "NCRust directory is already in User PATH." -ForegroundColor Green
}

Write-Host "Run NCRust by typing: ncrust" -ForegroundColor Green
