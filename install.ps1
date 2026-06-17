param (
    [switch]$DebugMode = $false,
    [switch]$Uninstall = $false
)

# Also check if "debug" or "uninstall" was passed as an unbound argument
if ($args -contains "debug") {
    $DebugMode = $true
}
if ($args -contains "uninstall") {
    $Uninstall = $true
}

$ErrorActionPreference = "Stop"

$repo = "FittyAr/Pairee"
$installDir = Join-Path $HOME "AppData\Local\Programs\pairee"
$configDir = Join-Path $env:APPDATA "pairee\config"
$exePath = Join-Path $installDir "pairee.exe"

# Uninstall logic
if ($Uninstall) {
    Write-Host "Pairee Uninstaller" -ForegroundColor Blue
    Write-Host "=============================="

    $userBin = Join-Path $installDir "pairee.exe"
    $sysBin1 = Join-Path $env:ProgramFiles "pairee\pairee.exe"
    $sysBin2 = Join-Path ${env:ProgramFiles(x86)} "pairee\pairee.exe"

    $installs = @()
    if (Test-Path $userBin) { $installs += @{ Id = 1; Path = $userBin; Desc = "User installation ($userBin)" } }
    if (Test-Path $sysBin1) { $installs += @{ Id = 2; Path = $sysBin1; Desc = "System-wide installation ($sysBin1)" } }
    if (Test-Path $sysBin2) { $installs += @{ Id = 3; Path = $sysBin2; Desc = "System-wide installation ($sysBin2)" } }

    if ($installs.Length -eq 0 -and -not (Test-Path $configDir)) {
        Write-Host "No Pairee installations or configurations found."
        exit 0
    }

    $toRemove = @()

    if ($installs.Length -eq 0) {
        Write-Host "No Pairee binaries found, but configuration folder exists."
    } elseif ($installs.Length -eq 1) {
        $activeInst = $installs[0]
        Write-Host "Found Pairee installed at: $($activeInst.Path)"
        $confirm = Read-Host "Do you want to uninstall it? [y/N]"
        if ($confirm -match "^[yY](es)?$") {
            $toRemove += $activeInst.Path
        } else {
            Write-Host "Uninstall cancelled."
            exit 0
        }
    } else {
        Write-Host "Multiple Pairee installations detected:"
        foreach ($inst in $installs) {
            Write-Host "  $($inst.Id)) $($inst.Desc)"
        }
        $selection = Read-Host "Enter the numbers you want to uninstall (e.g. '1', '1 2', or 'all') [Cancel]"
        if ($selection -match "^all$") {
            foreach ($inst in $installs) { $toRemove += $inst.Path }
        } elseif ($selection) {
            $ids = $selection -split '\s+'
            foreach ($id in $ids) {
                $match = $installs | Where-Object { $_.Id -eq [int]$id }
                if ($match) { $toRemove += $match.Path }
            }
        } else {
            Write-Host "Uninstall cancelled."
            exit 0
        }
    }

    foreach ($bin in $toRemove) {
        Write-Host "Removing binary: $bin"
        Remove-Item -Force $bin -ErrorAction SilentlyContinue
        
        $parent = Split-Path $bin
        if ((Split-Path $parent -Leaf) -eq "pairee") {
            if ((Get-ChildItem -Path $parent).Length -eq 0) {
                Write-Host "Removing empty parent folder: $parent"
                Remove-Item -Recurse -Force $parent -ErrorAction SilentlyContinue
            }
        }
    }

    if (Test-Path $configDir) {
        $confirmConfig = Read-Host "Do you want to delete the configuration, themes, and history settings at $configDir? [y/N]"
        if ($confirmConfig -match "^[yY](es)?$") {
            Write-Host "Removing configuration folder: $configDir"
            $configParent = Split-Path $configDir
            Remove-Item -Recurse -Force $configParent -ErrorAction SilentlyContinue
        } else {
            Write-Host "Keeping configuration settings."
        }
    }

    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -split ';' -contains $installDir) {
        Write-Host "Removing Pairee from User PATH..." -ForegroundColor Yellow
        $pathList = $userPath -split ';' | Where-Object { $_ -ne $installDir }
        $newPath = $pathList -join ';'
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Host "User PATH updated successfully." -ForegroundColor Green
    }

    Write-Host "=============================="
    Write-Host "Uninstall process completed successfully!" -ForegroundColor Green
    exit 0
}

Write-Host "Pairee Installer for Windows" -ForegroundColor Blue
Write-Host "=============================="

# Check for Existing Installation
if ((Test-Path $exePath) -or (Test-Path $configDir)) {
    Write-Host "Warning: Pairee is already installed." -ForegroundColor Yellow
    $overwrite = Read-Host "Do you want to overwrite and update the binary? [y/N]"
    if ($overwrite -notmatch "^[yY](es)?$") {
        Write-Host "Installation cancelled."
        exit 0
    }

    if (Test-Path $configDir) {
        $clearConfig = Read-Host "Do you want to clear old configurations, themes, and history settings? [y/N]"
        if ($clearConfig -match "^[yY](es)?$") {
            Write-Host "Clearing old settings in $configDir..."
            Remove-Item -Recurse -Force $configDir -ErrorAction SilentlyContinue
        } else {
            Write-Host "Keeping existing settings."
        }
    }
}

# 1. Dependency check for debug mode
if ($DebugMode) {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Write-Error "git is required for debug compilation but was not found in PATH."
        exit 1
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo/rust is required for debug compilation but was not found in PATH."
        exit 1
    }
}

# 2. Fetch Version
if ($DebugMode) {
    $version = "debug-source"
    Write-Host "Running in debug mode. Will compile from master branch source..." -ForegroundColor Green
} else {
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
}

# 3. Setup folders
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
}
if (-not (Test-Path (Join-Path $configDir "lang"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $configDir "lang") | Out-Null
}
if (-not (Test-Path (Join-Path $configDir "help"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $configDir "help") | Out-Null
}
if (-not (Test-Path (Join-Path $configDir "docs"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $configDir "docs") | Out-Null
}
if (-not (Test-Path (Join-Path $configDir "keymaps"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $configDir "keymaps") | Out-Null
}

# 4. Download and Extract ZIP (or Git Clone & Cargo Build in debug mode)
$tempDir = Join-Path $env:TEMP "pairee_install_$(Get-Date -Format 'yyyyMMddHHmmss')"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

if ($DebugMode) {
    Write-Host "Cloning repository..."
    $srcPath = Join-Path $tempDir "pairee_src"
    try {
        git clone --depth 1 "https://github.com/$repo.git" $srcPath
    } catch {
        Write-Error "Failed to clone repository: $_"
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        exit 1
    }

    Write-Host "Compiling Pairee (cargo build --release)..."
    Push-Location $srcPath
    try {
        cargo build --release
    } catch {
        Write-Error "Compilation failed: $_"
        Pop-Location
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        exit 1
    }
    Pop-Location

    $extractedFolder = $srcPath
    $binSrc = Join-Path $extractedFolder "target\release\pairee.exe"
} else {
    $zipName = "pairee-$version-x86_64-pc-windows-msvc.zip"
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

    $extractedFolder = Join-Path $tempDir "pairee-$version-x86_64-pc-windows-msvc"
    $binSrc = Join-Path $extractedFolder "pairee.exe"
}

# 5. Copying Files
Write-Host "Installing files..."

# Copy executable
Copy-Item -Path $binSrc -Destination $installDir -Force

# Copy resources
if (Test-Path (Join-Path $extractedFolder "lang")) {
    Copy-Item -Path (Join-Path $extractedFolder "lang\*") -Destination (Join-Path $configDir "lang") -Force -Recurse
}
if (Test-Path (Join-Path $extractedFolder "help")) {
    Copy-Item -Path (Join-Path $extractedFolder "help\*") -Destination (Join-Path $configDir "help") -Force -Recurse
}
if (Test-Path (Join-Path $extractedFolder "docs")) {
    Copy-Item -Path (Join-Path $extractedFolder "docs\*") -Destination (Join-Path $configDir "docs") -Force -Recurse
}
if (Test-Path (Join-Path $extractedFolder "keymaps")) {
    # Only copy preset files that do not already exist, to preserve user edits
    Get-ChildItem -Path (Join-Path $extractedFolder "keymaps") -Filter "*.toml" | ForEach-Object {
        $destFile = Join-Path $configDir "keymaps\$($_.Name)"
        if (-not (Test-Path $destFile)) {
            Copy-Item -Path $_.FullName -Destination $destFile -Force
        }
    }
}

# Clean up temp
Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

Write-Host "==============================" -ForegroundColor Blue
Write-Host "Pairee version $version installed successfully!" -ForegroundColor Green
Write-Host "Executable location: $installDir\pairee.exe" -ForegroundColor Blue
Write-Host "Configuration and resources folder: $configDir" -ForegroundColor Blue
Write-Host ""

# 6. PATH setup
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -split ';' -notcontains $installDir) {
    Write-Host "Adding Pairee directory to User PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
    # Update current process PATH
    $env:PATH = "$env:PATH;$installDir"
    Write-Host "PATH updated successfully. You might need to restart your terminal/IDE to refresh PATH changes." -ForegroundColor Green
} else {
    Write-Host "Pairee directory is already in User PATH." -ForegroundColor Green
}

Write-Host "Run Pairee by typing: pairee" -ForegroundColor Green
