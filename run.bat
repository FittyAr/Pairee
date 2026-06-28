@echo off
set GIT_DIR=
set GIT_WORK_TREE=
title Pairee Development & Test Shell
cls

:: 1. Check if Rustup/Cargo are available
where cargo >nul 2>nul
if %errorlevel%==0 goto postcargo
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" goto addcargo

echo [ERROR] Cargo is not installed or not in PATH.
echo Please ensure Rustup has completed installation.
pause
exit /b 1

:addcargo
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"

:postcargo

:: Locate git and cargo directories to preserve them in cleaned PATH
set "GIT_BIN_DIR="
for /f "delims=" %%i in ('where git 2^>nul') do set "GIT_BIN_DIR=%%~dpi"
set "CARGO_BIN_DIR="
for /f "delims=" %%i in ('where cargo 2^>nul') do set "CARGO_BIN_DIR=%%~dpi"

:: 2. Locate Visual Studio 2022 / 18 Build Tools or Community variables
set "VCVARS_PATH="
if exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if exist "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat"

if "%VCVARS_PATH%"=="" goto novcvars
echo [INFO] Loading VS environment: %VCVARS_PATH%
:: Clean up PATH temporarily to avoid "input line too long" error during VCVARS execution, but preserve git/cargo
set "ORIG_PATH=%PATH%"
set "PATH=C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0\"
if not "%CARGO_BIN_DIR%"=="" set "PATH=%PATH%;%CARGO_BIN_DIR%"
if not "%GIT_BIN_DIR%"=="" set "PATH=%PATH%;%GIT_BIN_DIR%"
call "%VCVARS_PATH%" x64 >nul
set "PATH=%PATH%;%ORIG_PATH%"
goto postvcvars

:novcvars
echo [WARNING] Could not locate VS 2022 vcvars64.bat script automatically.
echo Linking might fail if msvcrt.lib is not found.

:postvcvars


:menu
cls
echo ==========================================
echo       Pairee TUI Manager Helper Shell
echo ==========================================
echo  1. Run Pairee TUI (cargo run)
echo  2. Run Pairee TUI Standalone (cargo run -- --standalone)
echo  3. Run unit tests (cargo test)
echo  4. Run cargo check compiler validation
echo  5. Run clippy static checks (cargo clippy)
echo  6. Run format check (cargo fmt)
echo  7. Clean build directory (cargo clean)
echo  8. Install/Upgrade via WinGet
echo  9. Microsoft Store (MSIX) Developer Menu
echo  10. Bump version and publish release (Git Tag ^& Push)
echo  11. Exit
echo ==========================================
set /p opt="Choose an option (1-11): "

if "%opt%"=="1" (
    echo [INFO] Launching Pairee...
    cargo run
    pause
    goto menu
)
if "%opt%"=="2" (
    echo [INFO] Launching Pairee Standalone...
    cargo run -- --standalone
    pause
    goto menu
)
if "%opt%"=="3" (
    echo [INFO] Running cargo test...
    cargo test
    pause
    goto menu
)
if "%opt%"=="4" (
    echo [INFO] Running cargo check...
    cargo check
    pause
    goto menu
)
if "%opt%"=="5" (
    echo [INFO] Running cargo clippy...
    cargo clippy --all-targets -- -D warnings
    pause
    goto menu
)
if "%opt%"=="6" (
    echo [INFO] Running cargo fmt check...
    cargo fmt --all -- --check
    pause
    goto menu
)
if "%opt%"=="7" (
    echo [INFO] Cleaning workspace target...
    cargo clean
    pause
    goto menu
)
if "%opt%"=="8" (
    goto winget_menu
)
if "%opt%"=="9" (
    goto msix_menu
)
if "%opt%"=="10" (
    echo [INFO] Running bump version and release script...
    powershell -ExecutionPolicy Bypass -File "%~dp0scripts\bump_version.ps1"
    pause
    goto menu
)
if "%opt%"=="11" (
    exit /b 0
)

goto menu

:winget_menu
cls
echo ==========================================
echo       Install/Upgrade via WinGet
echo ==========================================
echo  1. Install Pairee (Auto-detect architecture)
echo  2. Install Pairee (Force x64)
echo  3. Install Pairee (Force ARM64)
echo  4. Upgrade Pairee to latest version
echo  5. Uninstall Pairee
echo  6. Back to main menu
echo ==========================================
set /p wg_opt="Choose an option (1-6): "
if "%wg_opt%"=="1" (
    echo [INFO] Installing Pairee...
    winget install FittyAr.Pairee --accept-source-agreements --accept-package-agreements
    pause
    goto winget_menu
)
if "%wg_opt%"=="2" (
    echo [INFO] Installing Pairee (x64)...
    winget install FittyAr.Pairee --architecture x64 --accept-source-agreements --accept-package-agreements
    pause
    goto winget_menu
)
if "%wg_opt%"=="3" (
    echo [INFO] Installing Pairee (ARM64)...
    winget install FittyAr.Pairee --architecture arm64 --accept-source-agreements --accept-package-agreements
    pause
    goto winget_menu
)
if "%wg_opt%"=="4" (
    echo [INFO] Upgrading Pairee...
    winget upgrade FittyAr.Pairee --accept-source-agreements --accept-package-agreements
    pause
    goto winget_menu
)
if "%wg_opt%"=="5" (
    echo [INFO] Uninstalling Pairee...
    winget uninstall FittyAr.Pairee
    pause
    goto winget_menu
)
if "%wg_opt%"=="6" (
    goto menu
)
goto winget_menu

:msix_menu
cls
echo ==========================================
echo      Microsoft Store (MSIX) Developer Menu
echo ==========================================
echo  1. Package MSIX package locally
echo  2. Create/Install self-signed testing cert
echo  3. Sign MSIX package locally
echo  4. Install local signed MSIX package
echo  5. Back to main menu
echo ==========================================
set /p mx_opt="Choose an option (1-5): "

if "%mx_opt%"=="1" (
    echo [INFO] Packaging MSIX...
    if not exist "target\release\pairee.exe" (
        echo [WARNING] target\release\pairee.exe not found. Compiling in release mode...
        cargo build --release
    )
    
    :: Create staging folder
    if exist "target\msix_staging" rd /s /q "target\msix_staging"
    mkdir "target\msix_staging"
    
    :: Copy files
    copy "target\release\pairee.exe" "target\msix_staging\"
    xcopy "lang" "target\msix_staging\lang\" /E /I /H /Y
    xcopy "help" "target\msix_staging\help\" /E /I /H /Y
    xcopy "manifests\msix\Assets" "target\msix_staging\Assets\" /E /I /H /Y
    copy "manifests\msix\AppxManifest.xml" "target\msix_staging\"
    
    :: Compile
    powershell -Command "
      $makeappx = (Get-ChildItem -Path 'C:\Program Files (x86)\Windows Kits\10\bin' -Filter 'makeappx.exe' -Recurse | Where-Object { \$_.FullName -match 'x64' } | Select-Object -First 1).FullName
      if (-not \$makeappx) { \$makeappx = 'makeappx.exe' }
      echo [INFO] Running: \$makeappx pack /d target\msix_staging /p target\pairee_local_x64.msix
      & \$makeappx pack /d target\msix_staging /p target\pairee_local_x64.msix /o
    "
    pause
    goto msix_menu
)
if "%mx_opt%"=="2" (
    echo [INFO] Creating and trusting self-signed certificate...
    powershell -Command "
      \$cert = New-SelfSignedCertificate -Type Custom -Subject 'CN=EDC5BDED-A726-42CD-B98E-5657B88D9832' -KeyUsage DigitalSignature -FriendlyName 'Pairee Local Test' -CertStoreLocation 'Cert:\CurrentUser\My' -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3')
      Export-Certificate -Cert \$cert -FilePath 'target\pairee_test.cer'
      Import-Certificate -FilePath 'target\pairee_test.cer' -CertStoreLocation 'Cert:\LocalMachine\Root'
      Write-Host '[SUCCESS] Certificate created at target\pairee_test.cer and trusted.'
    "
    pause
    goto msix_menu
)
if "%mx_opt%"=="3" (
    echo [INFO] Signing MSIX package...
    if not exist "target\pairee_local_x64.msix" (
        echo [ERROR] target\pairee_local_x64.msix not found. Package first.
        pause
        goto msix_menu
    )
    if not exist "target\pairee_test.cer" (
        echo [ERROR] target\pairee_test.cer not found. Create certificate first.
        pause
        goto msix_menu
    )
    powershell -Command "
      \$signtool = (Get-ChildItem -Path 'C:\Program Files (x86)\Windows Kits\10\bin' -Filter 'signtool.exe' -Recurse | Where-Object { \$_.FullName -match 'x64' } | Select-Object -First 1).FullName
      if (-not \$signtool) { \$signtool = 'signtool.exe' }
      echo [INFO] Running: \$signtool sign /fd SHA256 /a /f target\pairee_test.cer target\pairee_local_x64.msix
      & \$signtool sign /fd SHA256 /a /f target\pairee_test.cer target\pairee_local_x64.msix
    "
    pause
    goto msix_menu
)
if "%mx_opt%"=="4" (
    echo [INFO] Installing local signed MSIX package...
    if not exist "target\pairee_local_x64.msix" (
        echo [ERROR] target\pairee_local_x64.msix not found.
        pause
        goto msix_menu
    )
    powershell -Command "
      Add-AppxPackage -Path target\pairee_local_x64.msix
      Write-Host '[SUCCESS] Package installed.'
    "
    pause
    goto msix_menu
)
if "%mx_opt%"=="5" (
    goto menu
)
goto msix_menu
