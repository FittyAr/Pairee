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
    echo -e "  ${BOLD}9.${RESET} Bump version & release     ${YELLOW}(Git Tag & Push)${RESET}"
    echo -e "  ${BOLD}10.${RESET} Exit"
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

while true; do
    show_menu
    read -rp "Choose an option (1-10): " opt

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
            echo -e "\n${GREEN}[INFO]${RESET} Running bump version and release script...\n"
            run_and_pause "$SCRIPT_DIR/scripts/bump_version.sh"
            ;;
        10)
            echo -e "${CYAN}Bye!${RESET}"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}[WARN]${RESET} Invalid option. Please choose 1-10."
            sleep 1
            ;;
    esac
done
