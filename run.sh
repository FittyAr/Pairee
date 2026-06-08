#!/usr/bin/env bash
# =============================================================================
#  NCRust — Development & Test Shell  (Linux / Fedora)
#  Equivalent of run.bat adapted for bash on Fedora 44+
# =============================================================================

set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
RESET='\033[0m'

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
    echo -e "${CYAN}${BOLD}      NCRust TUI Manager Helper Shell     ${RESET}"
    echo -e "${CYAN}${BOLD}==========================================${RESET}"
    echo -e "  ${BOLD}1.${RESET} Run NCRust TUI            ${YELLOW}(cargo run)${RESET}"
    echo -e "  ${BOLD}2.${RESET} Run unit tests             ${YELLOW}(cargo test)${RESET}"
    echo -e "  ${BOLD}3.${RESET} Compiler validation        ${YELLOW}(cargo check)${RESET}"
    echo -e "  ${BOLD}4.${RESET} Static analysis            ${YELLOW}(cargo clippy -- -D warnings)${RESET}"
    echo -e "  ${BOLD}5.${RESET} Format check               ${YELLOW}(cargo fmt --all -- --check)${RESET}"
    echo -e "  ${BOLD}6.${RESET} Clean build directory      ${YELLOW}(cargo clean)${RESET}"
    echo -e "  ${BOLD}7.${RESET} Exit"
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

while true; do
    show_menu
    read -rp "Choose an option (1-7): " opt

    case "$opt" in
        1)
            echo -e "\n${GREEN}[INFO]${RESET} Launching NCRust...\n"
            run_and_pause cargo run
            ;;
        2)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo test...\n"
            run_and_pause cargo test
            ;;
        3)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo check...\n"
            run_and_pause cargo check
            ;;
        4)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo clippy...\n"
            run_and_pause cargo clippy --all-targets -- -D warnings
            ;;
        5)
            echo -e "\n${GREEN}[INFO]${RESET} Running cargo fmt check...\n"
            run_and_pause cargo fmt --all -- --check
            ;;
        6)
            echo -e "\n${GREEN}[INFO]${RESET} Cleaning workspace target...\n"
            run_and_pause cargo clean
            ;;
        7)
            echo -e "${CYAN}Bye!${RESET}"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}[WARN]${RESET} Invalid option. Please choose 1-7."
            sleep 1
            ;;
    esac
done
