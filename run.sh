#!/usr/bin/env bash
# =============================================================================
#  Pairee — Development & Test Shell  (Linux / Fedora)
#  Equivalent of run.bat adapted for bash on Fedora 44+
# =============================================================================

set -euo pipefail

ESC=$(printf '\033')
BOLD="${ESC}[1m"
GREEN="${ESC}[0;32m"
YELLOW="${ESC}[1;33m"
RED="${ESC}[0;31m"
CYAN="${ESC}[0;36m"
RESET="${ESC}[0m"

# ---------------------------------------------------------------------------
# 1. Locate Cargo — add ~/.cargo/bin to PATH if needed
# ---------------------------------------------------------------------------
if ! command -v cargo &>/dev/null; then
    if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo -e "${RED}[ERROR]${RESET} Cargo is not installed or not in PATH."
        echo "  Install Rust via: curl https://sh.rustup.rs -sSf | sh"
        exit 1
    fi
fi

echo -e "${GREEN}[INFO]${RESET} Cargo found: $(cargo --version)"

# ---------------------------------------------------------------------------
# 2. Ensure the script runs from the project root (where Cargo.toml lives)
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if [[ ! -f "Cargo.toml" ]]; then
    echo -e "${RED}[ERROR]${RESET} Cargo.toml not found in $SCRIPT_DIR"
    exit 1
fi

# ---------------------------------------------------------------------------
# 3. Interactive menu (mirrors run.bat behaviour)
# ---------------------------------------------------------------------------
show_menu() {
    clear
    echo -e "${CYAN}${BOLD}==========================================${RESET}"
    echo -e "${CYAN}${BOLD}      Pairee TUI Manager Helper Shell     ${RESET}"
    echo -e "${CYAN}${BOLD}==========================================${RESET}"
    echo -e "  ${BOLD}1.${RESET} Run Pairee TUI             ${YELLOW}(cargo run)${RESET}"
    echo -e "  ${BOLD}2.${RESET} Run Pairee TUI Standalone  ${YELLOW}(cargo run -- --standalone)${RESET}"
    echo -e "  ${BOLD}3.${RESET} Run unit tests             ${YELLOW}(cargo test)${RESET}"
    echo -e "  ${BOLD}4.${RESET} Compiler validation        ${YELLOW}(cargo check)${RESET}"
    echo -e "  ${BOLD}5.${RESET} Static analysis            ${YELLOW}(cargo clippy -- -D warnings)${RESET}"
    echo -e "  ${BOLD}6.${RESET} Format check               ${YELLOW}(cargo fmt --all -- --check)${RESET}"
    echo -e "  ${BOLD}7.${RESET} Clean build directory      ${YELLOW}(cargo clean)${RESET}"
    echo -e "  ${BOLD}8.${RESET} Install/Upgrade via WinGet  ${YELLOW}(winget)${RESET}"
    echo -e "  ${BOLD}9.${RESET} Microsoft Store (MSIX) Menu ${YELLOW}(msix)${RESET}"
    echo -e "  ${BOLD}10.${RESET} Bump version & release     ${YELLOW}(Git Tag & Push)${RESET}"
    echo -e "  ${BOLD}11.${RESET} Exit"
    echo -e "${CYAN}${BOLD}==========================================${RESET}"
}

run_and_pause() {
    # Run the supplied command; on failure print a clear message.
    if "$@"; then
        echo -e "\n${GREEN}[OK]${RESET} Command completed successfully."
    else
        echo -e "\n${RED}[FAILED]${RESET} Command exited with status $?."
    fi
    read -rp "Press ENTER to return to menu..." _
}

show_winget_menu() {
    # Check if winget or winget.exe is available
    local winget_cmd=""
    if command -v winget.exe &>/dev/null; then
        winget_cmd="winget.exe"
    elif command -v winget &>/dev/null; then
        winget_cmd="winget"
    fi

    if [[ -z "$winget_cmd" ]]; then
        clear
        echo -e "${RED}[ERROR]${RESET} WinGet (winget / winget.exe) was not found in your PATH."
        echo "WinGet is natively available on Windows. If you are on Linux, WinGet is not supported."
        echo ""
        read -rp "Press ENTER to return..." _
        return
    fi

    while true; do
        clear
        echo -e "${CYAN}${BOLD}==========================================${RESET}"
        echo -e "${CYAN}${BOLD}       Install/Upgrade via WinGet         ${RESET}"
        echo -e "${CYAN}${BOLD}==========================================${RESET}"
        echo -e "  ${BOLD}1.${RESET} Install Pairee (Auto-detect architecture)"
        echo -e "  ${BOLD}2.${RESET} Install Pairee (Force x64)"
        echo -e "  ${BOLD}3.${RESET} Install Pairee (Force ARM64)"
        echo -e "  ${BOLD}4.${RESET} Upgrade Pairee to latest version"
        echo -e "  ${BOLD}5.${RESET} Uninstall Pairee"
        echo -e "  ${BOLD}6.${RESET} Back to main menu"
        echo -e "${CYAN}${BOLD}==========================================${RESET}"
        read -rp "Choose an option (1-6): " wg_opt

        case "$wg_opt" in
            1)
                echo -e "\n${GREEN}[INFO]${RESET} Installing Pairee..."
                run_and_pause "$winget_cmd" install FittyAr.Pairee --accept-source-agreements --accept-package-agreements
                ;;
            2)
                echo -e "\n${GREEN}[INFO]${RESET} Installing Pairee (x64)..."
                run_and_pause "$winget_cmd" install FittyAr.Pairee --architecture x64 --accept-source-agreements --accept-package-agreements
                ;;
            3)
                echo -e "\n${GREEN}[INFO]${RESET} Installing Pairee (ARM64)..."
                run_and_pause "$winget_cmd" install FittyAr.Pairee --architecture arm64 --accept-source-agreements --accept-package-agreements
                ;;
            4)
                echo -e "\n${GREEN}[INFO]${RESET} Upgrading Pairee..."
                run_and_pause "$winget_cmd" upgrade FittyAr.Pairee --accept-source-agreements --accept-package-agreements
                ;;
            5)
                echo -e "\n${GREEN}[INFO]${RESET} Uninstalling Pairee..."
                run_and_pause "$winget_cmd" uninstall FittyAr.Pairee
                ;;
            6)
                break
                ;;
            *)
                echo -e "${YELLOW}[WARN]${RESET} Invalid option. Please choose 1-6."
                sleep 1
                ;;
        esac
    done
}

show_msix_menu() {
    # Check if we are running in a Windows environment
    local is_windows=false
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || -n "${WSL_DISTRO_NAME:-}" ]]; then
        is_windows=true
    fi

    if [[ "$is_windows" == "false" ]]; then
        clear
        echo -e "${RED}[ERROR]${RESET} MSIX packaging and deployment is Windows-specific."
        echo "Please run run.bat on Windows to build, sign, and test MSIX packages."
        echo ""
        read -rp "Press ENTER to return..." _
        return
    fi

    while true; do
        clear
        echo -e "${CYAN}${BOLD}==========================================${RESET}"
        echo -e "${CYAN}${BOLD}     Microsoft Store (MSIX) Developer Menu ${RESET}"
        echo -e "${CYAN}${BOLD}==========================================${RESET}"
        echo -e "  ${BOLD}1.${RESET} Package MSIX package locally"
        echo -e "  ${BOLD}2.${RESET} Create/Install self-signed testing cert"
        echo -e "  ${BOLD}3.${RESET} Sign MSIX package locally"
        echo -e "  ${BOLD}4.${RESET} Install local signed MSIX package"
        echo -e "  ${BOLD}5.${RESET} Back to main menu"
        echo -e "${CYAN}${BOLD}==========================================${RESET}"
        read -rp "Choose an option (1-5): " mx_opt

        case "$mx_opt" in
            1)
                echo -e "\n${GREEN}[INFO]${RESET} Packaging MSIX..."
                if [[ ! -f "target/release/pairee.exe" ]]; then
                    echo -e "${YELLOW}[WARNING]${RESET} target/release/pairee.exe not found. Compiling in release mode..."
                    run_and_pause cargo build --release
                fi

                # Staging
                rm -rf target/msix_staging
                mkdir -p target/msix_staging
                cp target/release/pairee.exe target/msix_staging/
                cp -r lang help manifests/msix/Assets target/msix_staging/
                cp manifests/msix/AppxManifest.xml target/msix_staging/

                powershell.exe -Command "
                  \$makeappx = (Get-ChildItem -Path 'C:\\Program Files (x86)\\Windows Kits\\10\\bin' -Filter 'makeappx.exe' -Recurse | Where-Object { \$_.FullName -match 'x64' } | Select-Object -First 1).FullName
                  if (-not \$makeappx) { \$makeappx = 'makeappx.exe' }
                  echo '[INFO] Running:' \$makeappx pack /d target\\msix_staging /p target\\pairee_local_x64.msix
                  & \$makeappx pack /d target\\msix_staging /p target\\pairee_local_x64.msix /o
                "
                read -rp "Press ENTER to return..." _
                ;;
            2)
                echo -e "\n${GREEN}[INFO]${RESET} Creating and trusting self-signed certificate..."
                powershell.exe -Command "
                  \$cert = New-SelfSignedCertificate -Type Custom -Subject 'CN=EDC5BDED-A726-42CD-B98E-5657B88D9832' -KeyUsage DigitalSignature -FriendlyName 'Pairee Local Test' -CertStoreLocation 'Cert:\\CurrentUser\\My' -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3')
                  Export-Certificate -Cert \$cert -FilePath 'target\\pairee_test.cer'
                  Import-Certificate -FilePath 'target\\pairee_test.cer' -CertStoreLocation 'Cert:\\LocalMachine\\Root'
                  Write-Host '[SUCCESS] Certificate created at target\\pairee_test.cer and trusted.'
                "
                read -rp "Press ENTER to return..." _
                ;;
            3)
                echo -e "\n${GREEN}[INFO]${RESET} Signing MSIX package..."
                if [[ ! -f "target/pairee_local_x64.msix" ]]; then
                    echo -e "${RED}[ERROR]${RESET} target/pairee_local_x64.msix not found. Package first."
                    read -rp "Press ENTER to return..." _
                    continue
                fi
                if [[ ! -f "target/pairee_test.cer" ]]; then
                    echo -e "${RED}[ERROR]${RESET} target/pairee_test.cer not found. Create certificate first."
                    read -rp "Press ENTER to return..." _
                    continue
                fi
                powershell.exe -Command "
                  \$signtool = (Get-ChildItem -Path 'C:\\Program Files (x86)\\Windows Kits\\10\\bin' -Filter 'signtool.exe' -Recurse | Where-Object { \$_.FullName -match 'x64' } | Select-Object -First 1).FullName
                  if (-not \$signtool) { \$signtool = 'signtool.exe' }
                  & \$signtool sign /fd SHA256 /a /f target\\pairee_test.cer target\\pairee_local_x64.msix
                "
                read -rp "Press ENTER to return..." _
                ;;
            4)
                echo -e "\n${GREEN}[INFO]${RESET} Installing local signed MSIX package..."
                if [[ ! -f "target/pairee_local_x64.msix" ]]; then
                    echo -e "${RED}[ERROR]${RESET} target/pairee_local_x64.msix not found."
                    read -rp "Press ENTER to return..." _
                    continue
                fi
                powershell.exe -Command "
                  Add-AppxPackage -Path target\\pairee_local_x64.msix
                  Write-Host '[SUCCESS] Package installed.'
                "
                read -rp "Press ENTER to return..." _
                ;;
            5)
                break
                ;;
            *)
                echo -e "${YELLOW}[WARN]${RESET} Invalid option. Please choose 1-5."
                sleep 1
                ;;
        esac
    done
}

while true; do
    show_menu
    read -rp "Choose an option (1-11): " opt

    case "$opt" in
        1)
            echo -e "\n${GREEN}[INFO]${RESET} Launching Pairee...\n"
            run_and_pause cargo run
            ;;
        2)
            echo -e "\n${GREEN}[INFO]${RESET} Launching Pairee (Standalone)...\n"
            run_and_pause cargo run -- --standalone
            ;;
        3)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo test...\n"
            run_and_pause cargo test
            ;;
        4)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo check...\n"
            run_and_pause cargo check
            ;;
        5)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo clippy...\n"
            run_and_pause cargo clippy --all-targets -- -D warnings
            ;;
        6)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo fmt check...\n"
            run_and_pause cargo fmt --all -- --check
            ;;
        7)
            echo -e "\n${GREEN}[INFO]${RESET} Cleaning workspace target...\n"
            run_and_pause cargo clean
            ;;
        8)
            show_winget_menu
            ;;
        9)
            show_msix_menu
            ;;
        10)
            echo -e "\n${GREEN}[INFO]${RESET} Running bump version and release script...\n"
            run_and_pause "$SCRIPT_DIR/scripts/bump_version.sh"
            ;;
        11)
            echo -e "${CYAN}Bye!${RESET}"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}[WARN]${RESET} Invalid option. Please choose 1-11."
            sleep 1
            ;;
    esac
done
