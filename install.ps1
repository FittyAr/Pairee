# Pairee installer for Windows (x64 and ARM64).
#
#   irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/FittyAr/Pairee/master/install.ps1))) [-DebugMode] [-Uninstall] [-InsecureSkipSignature] [-VerifyOnly]
#
# Installs pairee.exe to %LOCALAPPDATA%\Programs\FittyAr\Pairee and its resources
# to %APPDATA%\FittyAr\Pairee\config (all FittyAr apps share the FittyAr
# folders). Earlier layouts (Programs\pairee, %APPDATA%\pairee) are moved
# automatically (the settings stay in %APPDATA%\pairee while the release being
# installed is 0.11.0 or older, which still reads them there). The download is
# checked against its SHA-256 and
# its minisign signature. minisign itself is downloaded from its official
# release (jedisct1/minisign), pinned by version and SHA-256 below.
#   -InsecureSkipSignature  skip the signature (only the checksum, served from
#                           the same place as the archive, is checked)
#   -VerifyOnly             download and verify the release, install nothing
param (
    [switch]$DebugMode = $false,
    [switch]$Uninstall = $false,
    [switch]$InsecureSkipSignature = $false,
    [switch]$VerifyOnly = $false
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
# Pairee release signing key (minisign). Must match RELEASE_PUBLIC_KEY in
# src/update/signature.rs and install.sh (checked by tests/release_key.rs).
$minisignPubKey = "RWSF5Q18I/R4tADVZ5LQzgP2gRzPD/yzWj0p5kw13d6g4+Ycwbn27Fm6"
# minisign for Windows: https://github.com/jedisct1/minisign/releases
$minisignVersion = "0.12"
$minisignZipSha256 = "37b600344e20c19314b2e82813db2bfdcc408b77b876f7727889dbd46d539479"
$vendorDir = Join-Path $env:LOCALAPPDATA "Programs\FittyAr"
$installDir = Join-Path $vendorDir "Pairee"
$dataRoot = Join-Path $env:APPDATA "FittyAr\Pairee"
$configDir = Join-Path $dataRoot "config"
# Used by this script before installs moved under the FittyAr folders.
$previousDir = Join-Path $env:LOCALAPPDATA "Programs\pairee"
$previousDataRoot = Join-Path $env:APPDATA "pairee"
$exePath = Join-Path $installDir "pairee.exe"

# Whether this machine runs ARM64 Windows (also when this PowerShell itself
# runs emulated as x64).
function Test-Arm64 {
    $arch = $null
    try {
        $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    } catch {
        $arch = $null
    }
    if (-not $arch) {
        $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    }
    return $arch -match '^arm64$'
}

# Names of this machine's release archive, preferred first. The platform
# names come from src/update/assets.rs (checked by tests/release_assets.rs).
function Get-ArchiveNames([string]$tag) {
    $num = $tag.TrimStart('v')
    if (Test-Arm64) {
        $platform = "windows-arm64"
        $legacyTarget = "aarch64-pc-windows-msvc"
    } else {
        $platform = "windows-x64"
        $legacyTarget = "x86_64-pc-windows-msvc"
    }
    # TRANSITION: 0.11.0 (and every release before it) also
    # carry the archive under its pre-0.11 name; drop it, and $legacyTarget,
    # once a later release is the latest.
    return @("pairee-$num-$platform.zip", "pairee-$tag-$legacyTarget.zip")
}

# The user PATH as stored in the registry. [Environment]::GetEnvironmentVariable
# expands entries such as %USERPROFILE%\bin and SetEnvironmentVariable writes
# the result back as a plain string, so they would lose their variables.
function Get-UserPathEntries {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment")
    try {
        $raw = $key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally {
        $key.Close()
    }
    return @("$raw" -split ';' | Where-Object { $_ })
}

function Set-UserPathEntries([string[]]$entries) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
    try {
        $key.SetValue("Path", ($entries -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
    } finally {
        $key.Close()
    }
    # Setting a user variable through .NET broadcasts WM_SETTINGCHANGE, so
    # terminals opened from now on see the new PATH.
    [Environment]::SetEnvironmentVariable("PAIREE_PATH_REFRESH", "1", "User")
    [Environment]::SetEnvironmentVariable("PAIREE_PATH_REFRESH", $null, "User")
}

# Whether a PATH entry names $installDir (case, a trailing backslash and
# environment variables aside).
function Test-IsInstallDir([string]$entry) {
    return [Environment]::ExpandEnvironmentVariables($entry).TrimEnd('\') -eq $installDir.TrimEnd('\')
}

# Moves the pre-FittyAr settings folder into place: it is renamed only when
# the new one does not exist yet, and nothing is deleted. If it cannot be
# moved (a file in it is in use) it keeps being used, as Pairee does.
function Move-PreviousSettings {
    if (-not (Test-Path $dataRoot) -and (Test-Path $previousDataRoot)) {
        Write-Host "Moving settings from $previousDataRoot to $dataRoot..."
        New-Item -ItemType Directory -Force -Path (Split-Path $dataRoot) | Out-Null
        # A plain rename, all or nothing (Move-Item falls back to moving file
        # by file and could leave the settings split between both folders).
        try {
            [System.IO.Directory]::Move($previousDataRoot, $dataRoot)
        } catch {
            Write-Host "$($_.Exception.Message)" -ForegroundColor Yellow
        }
        if (-not (Test-Path $dataRoot)) {
            Write-Host "Could not move $previousDataRoot; it keeps being used." -ForegroundColor Yellow
            $vendor = Split-Path $dataRoot
            if (@(Get-ChildItem -Path $vendor -Force).Count -eq 0) {
                Remove-Item -Force $vendor -ErrorAction SilentlyContinue
            }
            $script:dataRoot = $previousDataRoot
            $script:configDir = Join-Path $previousDataRoot "config"
        }
    }
}

# Whether the Pairee being installed ($version) reads its settings from the
# FittyAr folder. TRANSITION: 0.11.0 and older read %APPDATA%\pairee; drop
# this (and its use in Select-SettingsFolder) once a newer release is the latest.
function Test-UsesFittyArFolders([string]$tag) {
    if ($tag -notmatch '^v(\d+)\.(\d+)\.(\d+)') { return $true }
    return [version]"$($Matches[1]).$($Matches[2]).$($Matches[3])" -gt [version]"0.11.0"
}

# The settings folder the installed version uses: the FittyAr one (after
# moving the previous folder into place) or, for 0.11.0 and older, the
# previous one.
function Select-SettingsFolder {
    if (Test-UsesFittyArFolders $script:version) {
        Move-PreviousSettings
    } else {
        $script:dataRoot = $previousDataRoot
        $script:configDir = Join-Path $previousDataRoot "config"
    }
}

# Removes the pre-FittyAr program folder and its PATH entry, once the new
# install is in place.
function Remove-PreviousProgram {
    if (Test-Path (Join-Path $previousDir "pairee.exe")) {
        Write-Host "Moving from $previousDir to $installDir..."
        Remove-Item -Recurse -Force $previousDir -ErrorAction SilentlyContinue
        $entries = Get-UserPathEntries
        $kept = @($entries | Where-Object {
            [Environment]::ExpandEnvironmentVariables($_).TrimEnd([char]92) -ne $previousDir.TrimEnd([char]92)
        })
        if ($kept.Count -ne $entries.Count) { Set-UserPathEntries $kept }
    }
}

# No `exit` here: run through `irm | iex`, it would close the user's shell.
function Invoke-Uninstall {
    Write-Host "Pairee Uninstaller" -ForegroundColor Blue
    Write-Host "=============================="
    # Nothing is moved here: a cancelled uninstall must leave the settings
    # where the installed version reads them.
    if (-not (Test-Path $dataRoot) -and (Test-Path $previousDataRoot)) {
        $script:dataRoot = $previousDataRoot
        $script:configDir = Join-Path $previousDataRoot "config"
    }

    $userBin = Join-Path $installDir "pairee.exe"
    $oldBin = Join-Path $previousDir "pairee.exe"
    $sysBin1 = Join-Path $env:ProgramFiles "pairee\pairee.exe"
    $sysBin2 = Join-Path ${env:ProgramFiles(x86)} "pairee\pairee.exe"

    $installs = @()
    if (Test-Path $userBin) { $installs += @{ Id = 1; Path = $userBin; Desc = "User installation ($userBin)" } }
    if (Test-Path $sysBin1) { $installs += @{ Id = 2; Path = $sysBin1; Desc = "System-wide installation ($sysBin1)" } }
    if (Test-Path $sysBin2) { $installs += @{ Id = 3; Path = $sysBin2; Desc = "System-wide installation ($sysBin2)" } }
    if (Test-Path $oldBin) { $installs += @{ Id = 4; Path = $oldBin; Desc = "Previous user installation ($oldBin)" } }

    if ($installs.Length -eq 0 -and -not (Test-Path $configDir)) {
        Write-Host "No Pairee installations or configurations found."
        return
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
            return
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
                $match = $installs | Where-Object { $_.Id -eq ($id -as [int]) }
                if ($match) { $toRemove += $match.Path }
            }
        } else {
            Write-Host "Uninstall cancelled."
            return
        }
    }

    foreach ($bin in $toRemove) {
        Write-Host "Removing binary: $bin"
        Remove-Item -Force $bin -ErrorAction SilentlyContinue
        
        $parent = Split-Path $bin
        if ((Split-Path $parent -Leaf) -in @("pairee", "Pairee")) {
            if (@(Get-ChildItem -Path $parent).Count -eq 0) {
                Write-Host "Removing empty parent folder: $parent"
                Remove-Item -Recurse -Force $parent -ErrorAction SilentlyContinue
            }
        }
        # Leave the shared FittyAr folder only while other apps use it.
        if ((Test-Path $vendorDir) -and @(Get-ChildItem -Path $vendorDir).Count -eq 0) {
            Remove-Item -Force $vendorDir -ErrorAction SilentlyContinue
        }
    }

    if (Test-Path $configDir) {
        $confirmConfig = Read-Host "Do you want to delete the configuration, themes, and history settings at ${configDir}? [y/N]"
        if ($confirmConfig -match "^[yY](es)?$") {
            Write-Host "Removing configuration folder: $dataRoot"
            Remove-Item -Recurse -Force $dataRoot, $previousDataRoot -ErrorAction SilentlyContinue
            # Leave the shared FittyAr folder only while other apps use it.
            $dataVendor = Split-Path $dataRoot
            if ((Test-Path $dataVendor) -and @(Get-ChildItem -Path $dataVendor).Count -eq 0) {
                Remove-Item -Force $dataVendor -ErrorAction SilentlyContinue
            }
        } else {
            Write-Host "Keeping configuration settings."
        }
    }

    $entries = Get-UserPathEntries
    $kept = @($entries | Where-Object { -not (Test-IsInstallDir $_) })
    if ($kept.Count -ne $entries.Count) {
        Write-Host "Removing Pairee from User PATH..." -ForegroundColor Yellow
        Set-UserPathEntries $kept
        Write-Host "User PATH updated successfully." -ForegroundColor Green
    }

    Write-Host "=============================="
    Write-Host "Uninstall process completed successfully!" -ForegroundColor Green
}

# Downloads the pinned minisign release into $dir and returns its minisign.exe.
function Get-Minisign([string]$dir) {
    $zip = Join-Path $dir "minisign-$minisignVersion-win64.zip"
    $url = "https://github.com/jedisct1/minisign/releases/download/$minisignVersion/minisign-$minisignVersion-win64.zip"
    Write-Host "Downloading minisign $minisignVersion..."
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    $actual = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLowerInvariant()
    if ($actual -ne $minisignZipSha256) {
        throw "The minisign download does not match its pinned SHA-256 ($actual)."
    }
    $out = Join-Path $dir "minisign"
    Expand-Archive -Path $zip -DestinationPath $out -Force
    $arch = if (Test-Arm64) { "aarch64" } else { "x86_64" }
    $exe = Join-Path $out "minisign-win64\$arch\minisign.exe"
    if (-not (Test-Path $exe)) {
        throw "minisign.exe not found in the minisign $minisignVersion archive."
    }
    return $exe
}

# Text of a small download (checksum, signature).
function Get-Text([string]$url) {
    $response = Invoke-WebRequest -Uri $url -UseBasicParsing
    if ($response.Content -is [byte[]]) {
        return [System.Text.Encoding]::UTF8.GetString($response.Content)
    }
    return [string]$response.Content
}

# Checks $file (published as $name at $url) against its .sha256 and, unless
# -InsecureSkipSignature, its .minisig, whose trusted comment must name $name.
function Test-Download([string]$file, [string]$name, [string]$url, [string]$tempDir) {
    Write-Host "Verifying SHA-256 checksum..."
    $expected = ((Get-Text "$url.sha256").Trim() -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -Path $file).Hash.ToLowerInvariant()
    if ($expected -notmatch '^[0-9a-f]{64}$' -or $expected -ne $actual) {
        throw "SHA-256 checksum mismatch for $name."
    }

    if ($InsecureSkipSignature) {
        Write-Host "Warning: -InsecureSkipSignature given; the minisign signature was not checked." -ForegroundColor Yellow
        return
    }
    $minisign = Get-Minisign $tempDir
    Write-Host "Verifying minisign signature..."
    $sigPath = "$file.minisig"
    Invoke-WebRequest -Uri "$url.minisig" -OutFile $sigPath -UseBasicParsing
    $sigOut = & $minisign -V -P $minisignPubKey -m $file -x $sigPath 2>&1 | Out-String
    $padded = " " + ($sigOut -replace "\s", " ") + " "
    if ($LASTEXITCODE -ne 0 -or -not $padded.Contains(" file:$name ")) {
        throw "minisign signature verification failed for $name."
    }
}

# Downloads, verifies and extracts the latest release into $tempDir; returns
# the extracted folder that holds pairee.exe.
function Get-Release([string]$tempDir) {
    Write-Host "Fetching latest version info..."
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -UseBasicParsing
    $script:version = $release.tag_name
    if ($script:version -notmatch '^v\d+\.\d+\.\d+[A-Za-z0-9.-]*$') {
        throw "Unexpected release tag '$($script:version)'."
    }
    Write-Host "Latest version found: $($script:version)" -ForegroundColor Green

    $published = @($release.assets | ForEach-Object { $_.name })
    $zipName = Get-ArchiveNames $script:version | Where-Object { $published -contains $_ } | Select-Object -First 1
    if (-not $zipName) {
        throw "Release $($script:version) has no Windows archive for this machine."
    }
    $url = "https://github.com/$repo/releases/download/$($script:version)/$zipName"
    $zipPath = Join-Path $tempDir $zipName
    Write-Host "Downloading $zipName..."
    Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    Test-Download $zipPath $zipName $url $tempDir

    Write-Host "Extracting archive..."
    $extractDir = Join-Path $tempDir "extract"
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
    $exe = Get-ChildItem -Path $extractDir -Recurse -Filter "pairee.exe" | Select-Object -First 1
    if (-not $exe) {
        throw "pairee.exe not found in $zipName."
    }
    return $exe.DirectoryName
}

# Clones and builds master in $tempDir; returns the source folder.
function Get-SourceBuild([string]$tempDir) {
    foreach ($tool in "git", "cargo") {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "$tool is required for debug compilation but was not found in PATH."
        }
    }
    $script:version = "debug-source"
    Write-Host "Running in debug mode. Will compile from master branch source..." -ForegroundColor Green
    $srcPath = Join-Path $tempDir "pairee_src"
    Write-Host "Cloning repository..."
    git clone --depth 1 "https://github.com/$repo.git" $srcPath
    if ($LASTEXITCODE -ne 0) { throw "Failed to clone repository." }

    Write-Host "Compiling Pairee (cargo build --release)..."
    Push-Location $srcPath
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "Compilation failed." }
    } finally {
        Pop-Location
    }
    Copy-Item -Path (Join-Path $srcPath "target\release\pairee.exe") -Destination $srcPath
    return $srcPath
}

# Asks before overwriting an existing installation (false: cancelled); may
# clear its settings.
function Confirm-Overwrite {
    $previousConfig = Join-Path $previousDataRoot "config"
    if (-not ((Test-Path $exePath) -or (Test-Path (Join-Path $previousDir "pairee.exe")) -or (Test-Path $configDir) -or (Test-Path $previousConfig))) {
        return $true
    }
    Write-Host "Warning: Pairee is already installed." -ForegroundColor Yellow
    $overwrite = Read-Host "Do you want to overwrite and update the binary? [y/N]"
    if ($overwrite -notmatch "^[yY](es)?$") {
        Write-Host "Installation cancelled."
        return $false
    }
    if ((Test-Path $configDir) -or (Test-Path $previousConfig)) {
        $clearAnswer = Read-Host "Do you want to clear old configurations, themes, and history settings? [y/N]"
        if ($clearAnswer -match "^[yY](es)?$") {
            # Deleted by Install-Files, once the download is verified.
            $script:clearConfig = $true
            Write-Host "Old settings will be cleared once the download is verified."
        } else {
            Write-Host "Keeping existing settings."
        }
    }
    return $true
}

# Copies pairee.exe and the resources of $package into place.
function Install-Files([string]$package) {
    Write-Host "Installing files..."
    Select-SettingsFolder
    if ($script:clearConfig) {
        Write-Host "Clearing old settings in $configDir..."
        Remove-Item -Recurse -Force $configDir -ErrorAction SilentlyContinue
    }
    foreach ($dir in $installDir, (Join-Path $configDir "lang"), (Join-Path $configDir "help")) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
    Copy-Item -Path (Join-Path $package "pairee.exe") -Destination $installDir -Force

    if (Test-Path (Join-Path $package "lang")) {
        Copy-Item -Path (Join-Path $package "lang\*") -Destination (Join-Path $configDir "lang") -Force -Recurse
        # Clean up legacy JSON translation files
        Remove-Item -Path (Join-Path $configDir "lang\*.json") -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path (Join-Path $package "help")) {
        Copy-Item -Path (Join-Path $package "help\*") -Destination (Join-Path $configDir "help") -Force -Recurse
    }
}

function Add-ToUserPath {
    $entries = Get-UserPathEntries
    if (-not @($entries | Where-Object { Test-IsInstallDir $_ })) {
        Write-Host "Adding Pairee directory to User PATH..." -ForegroundColor Yellow
        Set-UserPathEntries (@($entries) + $installDir)
        # Update current process PATH
        $env:PATH = "$env:PATH;$installDir"
        Write-Host "PATH updated successfully. You might need to restart your terminal/IDE to refresh PATH changes." -ForegroundColor Green
    } else {
        Write-Host "Pairee directory is already in User PATH." -ForegroundColor Green
    }
}

# No `exit` here: run through `irm | iex`, it would close the user's shell.
function Invoke-Install {
    Write-Host "Pairee Installer for Windows" -ForegroundColor Blue
    Write-Host "=============================="
    if (-not $VerifyOnly -and -not (Confirm-Overwrite)) {
        return
    }
    $tempDir = Join-Path $env:TEMP "pairee_install_$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
    try {
        $package = if ($DebugMode) { Get-SourceBuild $tempDir } else { Get-Release $tempDir }
        if ($VerifyOnly) {
            Write-Host "Pairee $version downloaded and verified; nothing was installed (-VerifyOnly)." -ForegroundColor Green
            return
        }
        Install-Files $package
        Remove-PreviousProgram
    } catch {
        Write-Host "Error: $($_.Exception.Message) Aborting." -ForegroundColor Red
        throw
    } finally {
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
    }

    Write-Host "==============================" -ForegroundColor Blue
    Write-Host "Pairee version $version installed successfully!" -ForegroundColor Green
    Write-Host "Executable location: $installDir\pairee.exe" -ForegroundColor Blue
    Write-Host "Configuration and resources folder: $configDir" -ForegroundColor Blue
    Write-Host ""
    Add-ToUserPath
    Write-Host "Run Pairee by typing: pairee" -ForegroundColor Green
}

if ($Uninstall) {
    Invoke-Uninstall
} else {
    Invoke-Install
}
