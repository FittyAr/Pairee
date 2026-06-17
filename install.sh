#!/bin/sh
set -e

# Pairee Linux Installer
# Installs Pairee statically built binary and copies assets to the user's config directories.

REPO="FittyAr/Pairee"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/pairee"

# Parse arguments
DEBUG_MODE=false
UNINSTALL_MODE=false
for arg in "$@"; do
    if [ "$arg" = "debug" ]; then
        DEBUG_MODE=true
    elif [ "$arg" = "uninstall" ]; then
        UNINSTALL_MODE=true
    fi
done

# Colors
ESC=$(printf '\033')
RED="${ESC}[0;31m"
GREEN="${ESC}[0;32m"
BLUE="${ESC}[0;34m"
YELLOW="${ESC}[1;33m"
NC="${ESC}[0m" # No Color

# 3. Uninstall logic
if [ "$UNINSTALL_MODE" = "true" ]; then
    echo "${BLUE}Pairee Uninstaller${NC}"
    echo "=============================="

    USER_BIN="$HOME/.local/bin/pairee"
    SYS_BIN_1="/usr/bin/pairee"
    SYS_BIN_2="/usr/local/bin/pairee"

    INSTALLATIONS=""
    [ -f "$USER_BIN" ] && INSTALLATIONS="$INSTALLATIONS 1"
    [ -f "$SYS_BIN_1" ] && INSTALLATIONS="$INSTALLATIONS 2"
    [ -f "$SYS_BIN_2" ] && INSTALLATIONS="$INSTALLATIONS 3"

    if [ -z "$INSTALLATIONS" ] && [ ! -d "$CONFIG_DIR" ]; then
        echo "No Pairee installations or configurations found."
        exit 0
    fi

    TO_REMOVE=""
    NUM_INST=0
    for i in $INSTALLATIONS; do
        NUM_INST=$((NUM_INST + 1))
    done

    if [ $NUM_INST -eq 0 ]; then
        echo "No Pairee binaries found, but configuration folder exists."
    elif [ $NUM_INST -eq 1 ]; then
        ACTIVE_BIN=""
        [ -f "$USER_BIN" ] && ACTIVE_BIN="$USER_BIN"
        [ -f "$SYS_BIN_1" ] && ACTIVE_BIN="$SYS_BIN_1"
        [ -f "$SYS_BIN_2" ] && ACTIVE_BIN="$SYS_BIN_2"

        printf "Found Pairee installed at: %s\n" "$ACTIVE_BIN"
        printf "Do you want to uninstall it? [y/N]: "
        read -r CONFIRM < /dev/tty || CONFIRM="n"
        if [ "$CONFIRM" = "y" ] || [ "$CONFIRM" = "Y" ]; then
            TO_REMOVE="$ACTIVE_BIN"
        else
            echo "Uninstall cancelled."
            exit 0
        fi
    else
        echo "Multiple Pairee installations detected:"
        [ -f "$USER_BIN" ] && echo "  1) User installation ($USER_BIN)"
        [ -f "$SYS_BIN_1" ] && echo "  2) System-wide installation ($SYS_BIN_1)"
        [ -f "$SYS_BIN_2" ] && echo "  3) System-wide installation ($SYS_BIN_2)"

        printf "Enter the numbers you want to uninstall (e.g. '1', '1 2', or 'all') [Cancel]: "
        read -r SELECTION < /dev/tty || SELECTION=""
        
        if [ "$SELECTION" = "all" ] || [ "$SELECTION" = "ALL" ]; then
            [ -f "$USER_BIN" ] && TO_REMOVE="$TO_REMOVE $USER_BIN"
            [ -f "$SYS_BIN_1" ] && TO_REMOVE="$TO_REMOVE $SYS_BIN_1"
            [ -f "$SYS_BIN_2" ] && TO_REMOVE="$TO_REMOVE $SYS_BIN_2"
        elif [ -n "$SELECTION" ]; then
            for sel in $SELECTION; do
                case "$sel" in
                    1) [ -f "$USER_BIN" ] && TO_REMOVE="$TO_REMOVE $USER_BIN" ;;
                    2) [ -f "$SYS_BIN_1" ] && TO_REMOVE="$TO_REMOVE $SYS_BIN_1" ;;
                    3) [ -f "$SYS_BIN_2" ] && TO_REMOVE="$TO_REMOVE $SYS_BIN_2" ;;
                esac
            done
        else
            echo "Uninstall cancelled."
            exit 0
        fi
    fi

    for bin in $TO_REMOVE; do
        echo "Removing binary: $bin"
        if [ "$bin" = "$SYS_BIN_1" ] || [ "$bin" = "$SYS_BIN_2" ]; then
            if [ "$(id -u)" -ne 0 ]; then
                echo "Requesting root privileges to remove system binary..."
                sudo rm -f "$bin"
            else
                rm -f "$bin"
            fi
        else
            rm -f "$bin"
        fi
    done

    if [ -d "$CONFIG_DIR" ]; then
        printf "Do you want to delete the configuration, themes, and history settings at %s? [y/N]: " "$CONFIG_DIR"
        read -r CONFIRM_CONFIG < /dev/tty || CONFIRM_CONFIG="n"
        if [ "$CONFIRM_CONFIG" = "y" ] || [ "$CONFIRM_CONFIG" = "Y" ]; then
            echo "Removing configuration folder: $CONFIG_DIR"
            rm -rf "$CONFIG_DIR"
        else
            echo "Keeping configuration settings."
        fi
    fi

    echo "=============================="
    echo "${GREEN}Uninstall process completed successfully!${NC}"
    exit 0
fi

echo "${BLUE}Pairee Installer for Linux${NC}"
echo "=============================="

# 1. Architecture Check
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
    echo "${RED}Error: This script only supports Linux.${NC}"
    exit 1
fi

if [ "$ARCH" != "x86_64" ]; then
    echo "${RED}Error: Currently only x86_64 architecture is supported via installer.${NC}"
    exit 1
fi

# 2. Dependency Check
DEPENDENCIES="curl tar"
if [ "$DEBUG_MODE" = "true" ]; then
    DEPENDENCIES="git cargo $DEPENDENCIES"
fi
for cmd in $DEPENDENCIES; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "${RED}Error: Required command '$cmd' is not installed.${NC}"
        exit 1
    fi
done

# 3. Check for Existing Installation
if [ -f "$INSTALL_DIR/pairee" ] || [ -d "$CONFIG_DIR" ]; then
    echo "${YELLOW}Warning: Pairee is already installed.${NC}"
    
    if [ -c /dev/tty ]; then
        printf "Do you want to overwrite and update the binary? [y/N]: "
        read -r OVERWRITE < /dev/tty || OVERWRITE="n"
        case "$OVERWRITE" in
            [yY][eE][sS]|[yY])
                echo "Proceeding with update..."
                ;;
            *)
                echo "Installation cancelled."
                exit 0
                ;;
        esac

        if [ -d "$CONFIG_DIR" ]; then
            printf "Do you want to clear old configurations, themes, and history settings? [y/N]: "
            read -r CLEAR_CONFIG < /dev/tty || CLEAR_CONFIG="n"
            case "$CLEAR_CONFIG" in
                [yY][eE][sS]|[yY])
                    echo "Clearing old settings in $CONFIG_DIR..."
                    rm -rf "$CONFIG_DIR"
                    ;;
                *)
                    echo "Keeping existing settings."
                    ;;
            esac
        fi
    else
        echo "Non-interactive shell detected. Overwriting existing installation..."
    fi
fi

# 4. Retrieve Latest Version
if [ "$DEBUG_MODE" = "true" ]; then
    VERSION="debug-source"
    echo "Running in debug mode. Will compile from master branch source..."
else
    echo "Fetching latest version info..."
    VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | head -n 1 | cut -d '"' -f 4)

    if [ -z "$VERSION" ]; then
        echo "${RED}Error: Could not retrieve latest release version from GitHub API.${NC}"
        exit 1
    fi
    echo "Latest version found: ${GREEN}${VERSION}${NC}"
fi

# 5. Create paths
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR/lang"
mkdir -p "$CONFIG_DIR/help"
mkdir -p "$CONFIG_DIR/docs"

# 6. Download and Extract (or Git Clone & Cargo Build in debug mode)
TEMP_DIR=$(mktemp -d)

if [ "$DEBUG_MODE" = "true" ]; then
    echo "Cloning repository..."
    git clone --depth 1 "https://github.com/${REPO}.git" "${TEMP_DIR}/pairee_src"

    echo "Compiling Pairee (cargo build --release)..."
    # Execute cargo build and handle failures
    if ! (cd "${TEMP_DIR}/pairee_src" && cargo build --release); then
        echo "${RED}Error: Compilation failed.${NC}"
        rm -rf "$TEMP_DIR"
        exit 1
    fi

    PKG_FOLDER="${TEMP_DIR}/pairee_src"
    BIN_SRC="${PKG_FOLDER}/target/release/pairee"
else
    TARBALL="pairee-${VERSION}-x86_64-unknown-linux-musl.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL}"

    echo "Downloading ${TARBALL}..."
    curl -L "$DOWNLOAD_URL" -o "${TEMP_DIR}/${TARBALL}"

    echo "Extracting archive..."
    tar -xzf "${TEMP_DIR}/${TARBALL}" -C "$TEMP_DIR"

    PKG_FOLDER="${TEMP_DIR}/pairee-${VERSION}-x86_64-unknown-linux-musl"
    BIN_SRC="${PKG_FOLDER}/pairee"
fi

# 7. Install assets and binary
echo "Installing files..."

# Copy binary
cp "$BIN_SRC" "$INSTALL_DIR/pairee"
chmod +x "$INSTALL_DIR/pairee"

# Copy translations, help markdown and docs
if [ -d "${PKG_FOLDER}/lang" ]; then
    cp -r "${PKG_FOLDER}/lang/"* "$CONFIG_DIR/lang/"
fi
if [ -d "${PKG_FOLDER}/help" ]; then
    cp -r "${PKG_FOLDER}/help/"* "$CONFIG_DIR/help/"
fi
if [ -d "${PKG_FOLDER}/docs" ]; then
    cp -r "${PKG_FOLDER}/docs/"* "$CONFIG_DIR/docs/"
fi

# Clean up
rm -rf "$TEMP_DIR"

echo "=============================="
echo "${GREEN}Pairee version ${VERSION} installed successfully!${NC}"
echo "Binary location: ${BLUE}${INSTALL_DIR}/pairee${NC}"
echo "Config location: ${BLUE}${CONFIG_DIR}/${NC}"
echo ""

# 8. PATH verification
case :$PATH: in
    *:"$INSTALL_DIR":*) ;;
    *)
        echo "${BLUE}Note: '${INSTALL_DIR}' is not in your PATH.${NC}"
        echo "Please add it to your shell configuration (e.g. ~/.bashrc or ~/.zshrc):"
        echo "  ${GREEN}export PATH=\"\$PATH:\$HOME/.local/bin\"${NC}"
        echo ""
        ;;
esac

echo "Run Pairee by typing: ${GREEN}pairee${NC}"
