@echo off
title NCRust Development & Test Shell
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

:: 2. Locate Visual Studio 2022 / 18 Build Tools or Community variables
set "VCVARS_PATH="
if exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if exist "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" set "VCVARS_PATH=C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat"

if "%VCVARS_PATH%"=="" goto novcvars
echo [INFO] Loading VS environment: %VCVARS_PATH%
:: Clean up PATH temporarily to avoid "input line too long" error during VCVARS execution
set "PATH=C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0\;%USERPROFILE%\.cargo\bin"
call "%VCVARS_PATH%" x64 >nul
goto postvcvars

:novcvars
echo [WARNING] Could not locate VS 2022 vcvars64.bat script automatically.
echo Linking might fail if msvcrt.lib is not found.

:postvcvars


:menu
cls
echo ==========================================
echo       NCRust TUI Manager Helper Shell
echo ==========================================
echo  1. Run NCRust TUI (cargo run)
echo  2. Run unit tests (cargo test)
echo  3. Run cargo check compiler validation
echo  4. Run clippy static checks (cargo clippy)
echo  5. Clean build directory (cargo clean)
echo  6. Exit
echo ==========================================
set /p opt="Choose an option (1-6): "

if "%opt%"=="1" (
    echo [INFO] Launching NCRust...
    cargo run
    pause
    goto menu
)
if "%opt%"=="2" (
    echo [INFO] Running cargo test...
    cargo test
    pause
    goto menu
)
if "%opt%"=="3" (
    echo [INFO] Running cargo check...
    cargo check
    pause
    goto menu
)
if "%opt%"=="4" (
    echo [INFO] Running cargo clippy...
    cargo clippy --all-targets -- -D warnings
    pause
    goto menu
)
if "%opt%"=="5" (
    echo [INFO] Cleaning workspace target...
    cargo clean
    pause
    goto menu
)
if "%opt%"=="6" (
    exit /b 0
)

goto menu
