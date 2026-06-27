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
echo  9. Bump version and publish release (Git Tag & Push)
echo  10. Exit
echo ==========================================
set /p opt="Choose an option (1-10): "

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
    echo [INFO] Running bump version and release script...
    powershell -ExecutionPolicy Bypass -File "%~dp0scripts\bump_version.ps1"
    pause
    goto menu
)
if "%opt%"=="10" (
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
if "%wg_opt%"=="6" goto menu
goto winget_menu
