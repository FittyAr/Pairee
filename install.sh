#!/bin/sh
set -e

# NCRust Linux Installer
# Installs NCRust statically built binary and copies assets to the user's config directories.

REPO="FittyAr/NCRust"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/NCRust"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "${BLUE}NCRust Installer for Linux${NC}"
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
for cmd in curl tar; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "${RED}Error: Required command '$cmd' is not installed.${NC}"
        exit 1
    fi
done

# 3. Retrieve Latest Version
echo "Fetching latest version info..."
VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | head -n 1 | cut -d '"' -f 4)

if [ -z "$VERSION" ]; then
    echo "${RED}Error: Could not retrieve latest release version from GitHub API.${NC}"
    exit 1
fi
echo "Latest version found: ${GREEN}${VERSION}${NC}"

# 4. Create paths
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR/lang"
mkdir -p "$CONFIG_DIR/help"

# 5. Download and Extract
TEMP_DIR=$(mktemp -d)
TARBALL="ncrust-${VERSION}-x86_64-unknown-linux-musl.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL}"

echo "Downloading ${TARBALL}..."
curl -L "$DOWNLOAD_URL" -o "${TEMP_DIR}/${TARBALL}"

echo "Extracting archive..."
tar -xzf "${TEMP_DIR}/${TARBALL}" -C "$TEMP_DIR"

# 6. Install assets and binary
echo "Installing files..."
PKG_FOLDER="${TEMP_DIR}/ncrust-${VERSION}-x86_64-unknown-linux-musl"

# Copy binary
cp "${PKG_FOLDER}/ncrust" "$INSTALL_DIR/ncrust"
chmod +x "$INSTALL_DIR/ncrust"

# Copy translations and help markdown
cp -r "${PKG_FOLDER}/lang/"* "$CONFIG_DIR/lang/"
cp -r "${PKG_FOLDER}/help/"* "$CONFIG_DIR/help/"

# Clean up
rm -rf "$TEMP_DIR"

echo "=============================="
echo "${GREEN}NCRust version ${VERSION} installed successfully!${NC}"
echo "Binary location: ${BLUE}${INSTALL_DIR}/ncrust${NC}"
echo "Config location: ${BLUE}${CONFIG_DIR}/${NC}"
echo ""

# 7. PATH verification
case :$PATH: in
    *:"$INSTALL_DIR":*) ;;
    *)
        echo "${BLUE}Note: '${INSTALL_DIR}' is not in your PATH.${NC}"
        echo "Please add it to your shell configuration (e.g. ~/.bashrc or ~/.zshrc):"
        echo "  ${GREEN}export PATH=\"\$PATH:\$HOME/.local/bin\"${NC}"
        echo ""
        ;;
esac

echo "Run NCRust by typing: ${GREEN}ncrust${NC}"
