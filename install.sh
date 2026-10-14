#!/bin/sh
# Pairee installer for Linux (x86_64 and aarch64).
#
#   curl -fsSL https://raw.githubusercontent.com/FittyAr/Pairee/master/install.sh | sh
#   ... | sh -s -- [debug] [uninstall] [--insecure-skip-signature] [--verify-only]
#
# Installs the static (musl) binary to ~/.local/bin and its resources to
# ${XDG_CONFIG_HOME:-~/.config}/pairee. The download is checked against its
# SHA-256 and its minisign signature. When minisign is not installed, its
# official release (jedisct1/minisign) is downloaded into the temporary
# folder, pinned by version and SHA-256 below. --insecure-skip-signature
# checks only the checksum, served from the same place as the archive. --verify-only downloads and verifies
# the release and installs nothing.
#
# Everything runs from `main` on the last line, so a download cut short
# executes nothing.
set -eu

REPO="FittyAr/Pairee"
# Pairee release signing key (minisign). Must match RELEASE_PUBLIC_KEY in
# src/update/signature.rs and install.ps1 (checked by tests/release_key.rs).
MINISIGN_PUBKEY="RWSF5Q18I/R4tADVZ5LQzgP2gRzPD/yzWj0p5kw13d6g4+Ycwbn27Fm6"
# minisign for Linux, used when it is not installed (same version as in
# install.ps1): https://github.com/jedisct1/minisign/releases
MINISIGN_VERSION="0.12"
MINISIGN_TARBALL_SHA256="9a599b48ba6eb7b1e80f12f36b94ceca7c00b7a5173c95c3efc88d9822957e73"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/pairee"
# Where Pairee keeps its log and the plugins' data (removed on uninstall
# together with the configuration, when the user agrees).
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/pairee"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/pairee"

DEBUG_MODE=false
UNINSTALL_MODE=false
SKIP_SIGNATURE=false
VERIFY_ONLY=false
# Removed on exit, whatever the outcome.
TEMP_DIR=""
STAGED_BIN=""

ESC=$(printf '\033')
RED="${ESC}[0;31m"
GREEN="${ESC}[0;32m"
BLUE="${ESC}[0;34m"
YELLOW="${ESC}[1;33m"
NC="${ESC}[0m"

die() {
    echo "${RED}Error: $*${NC}" >&2
    exit 1
}

cleanup() {
    if [ -n "$STAGED_BIN" ]; then
        rm -f "$STAGED_BIN"
    fi
    if [ -n "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}

parse_args() {
    for arg in "$@"; do
        case "$arg" in
            debug | --debug) DEBUG_MODE=true ;;
            uninstall | --uninstall) UNINSTALL_MODE=true ;;
            --insecure-skip-signature) SKIP_SIGNATURE=true ;;
            --verify-only) VERIFY_ONLY=true ;;
            *) die "Unknown argument '$arg'. Usage: install.sh [debug] [uninstall] [--insecure-skip-signature] [--verify-only]" ;;
        esac
    done
}

# ─── Uninstall ───────────────────────────────────────────────────────────────

remove_binary() {
    echo "Removing binary: $1"
    case "$1" in
        "$HOME"/*) rm -f "$1" ;;
        *)
            if [ "$(id -u)" -ne 0 ]; then
                echo "Requesting root privileges to remove system binary..."
                sudo rm -f "$1"
            else
                rm -f "$1"
            fi
            ;;
    esac
}

uninstall() {
    echo "${BLUE}Pairee Uninstaller${NC}"
    echo "=============================="

    USER_BIN="$INSTALL_DIR/pairee"
    SYS_BIN_1="/usr/bin/pairee"
    SYS_BIN_2="/usr/local/bin/pairee"

    FOUND=""
    for bin in "$USER_BIN" "$SYS_BIN_1" "$SYS_BIN_2"; do
        if [ -f "$bin" ]; then
            FOUND="$FOUND $bin"
        fi
    done

    if [ -z "$FOUND" ] && [ ! -d "$CONFIG_DIR" ]; then
        echo "No Pairee installations or configurations found."
        return 0
    fi

    TO_REMOVE=""
    # shellcheck disable=SC2086 # split the list into words on purpose
    set -- $FOUND
    if [ $# -eq 0 ]; then
        echo "No Pairee binaries found, but configuration folder exists."
    elif [ $# -eq 1 ]; then
        printf "Found Pairee installed at: %s\n" "$1"
        printf "Do you want to uninstall it? [y/N]: "
        read -r CONFIRM < /dev/tty || CONFIRM="n"
        case "$CONFIRM" in
            [yY] | [yY][eE][sS]) TO_REMOVE="$1" ;;
            *)
                echo "Uninstall cancelled."
                return 0
                ;;
        esac
    else
        echo "Multiple Pairee installations detected:"
        if [ -f "$USER_BIN" ]; then echo "  1) User installation ($USER_BIN)"; fi
        if [ -f "$SYS_BIN_1" ]; then echo "  2) System-wide installation ($SYS_BIN_1)"; fi
        if [ -f "$SYS_BIN_2" ]; then echo "  3) System-wide installation ($SYS_BIN_2)"; fi

        printf "Enter the numbers you want to uninstall (e.g. '1', '1 2', or 'all') [Cancel]: "
        read -r SELECTION < /dev/tty || SELECTION=""
        case "$SELECTION" in
            all | ALL) SELECTION="1 2 3" ;;
            "")
                echo "Uninstall cancelled."
                return 0
                ;;
        esac
        for sel in $SELECTION; do
            case "$sel" in
                1) bin="$USER_BIN" ;;
                2) bin="$SYS_BIN_1" ;;
                3) bin="$SYS_BIN_2" ;;
                *) continue ;;
            esac
            if [ -f "$bin" ]; then
                TO_REMOVE="$TO_REMOVE $bin"
            fi
        done
    fi

    for bin in $TO_REMOVE; do
        remove_binary "$bin"
    done

    if [ -d "$CONFIG_DIR" ]; then
        printf "Do you want to delete the configuration, themes, and history settings at %s (and the logs and plugin data)? [y/N]: " "$CONFIG_DIR"
        read -r CONFIRM_CONFIG < /dev/tty || CONFIRM_CONFIG="n"
        case "$CONFIRM_CONFIG" in
            [yY] | [yY][eE][sS])
                echo "Removing configuration folder: $CONFIG_DIR"
                echo "Removing logs and plugin data: $CACHE_DIR $DATA_DIR"
                rm -rf "$CONFIG_DIR" "$CACHE_DIR" "$DATA_DIR"
                ;;
            *) echo "Keeping configuration settings." ;;
        esac
    fi

    echo "=============================="
    echo "${GREEN}Uninstall process completed successfully!${NC}"
}

# ─── Install ─────────────────────────────────────────────────────────────────

# Release platform of this machine (the static musl builds). The platform
# names come from src/update/assets.rs (checked by tests/release_assets.rs).
detect_platform() {
    [ "$(uname -s)" = "Linux" ] || die "This script only supports Linux."
    case "$(uname -m)" in
        x86_64 | amd64)
            PLATFORM="linux-x64-musl"
            MINISIGN_ARCH="x86_64"
            LEGACY_TARGET="x86_64-unknown-linux-musl"
            ;;
        aarch64 | arm64)
            PLATFORM="linux-arm64-musl"
            MINISIGN_ARCH="aarch64"
            LEGACY_TARGET="aarch64-unknown-linux-musl"
            ;;
        *) die "Unsupported architecture '$(uname -m)'. Release builds exist for x86_64 and aarch64." ;;
    esac
}

check_dependencies() {
    DEPENDENCIES="curl tar"
    if [ "$DEBUG_MODE" = "true" ]; then
        DEPENDENCIES="git cargo $DEPENDENCIES"
    fi
    for cmd in $DEPENDENCIES; do
        command -v "$cmd" > /dev/null 2>&1 || die "Required command '$cmd' is not installed."
    done
    if [ "$DEBUG_MODE" = "true" ]; then
        return 0
    fi
    if ! command -v sha256sum > /dev/null 2>&1 && ! command -v shasum > /dev/null 2>&1; then
        die "'sha256sum' or 'shasum' is required to verify the download."
    fi
}

# The minisign to verify with: the installed one, else the pinned official
# release downloaded into $TEMP_DIR (checked against its SHA-256).
find_minisign() {
    if command -v minisign > /dev/null 2>&1; then
        MINISIGN=minisign
        return 0
    fi
    echo "minisign is not installed; downloading minisign ${MINISIGN_VERSION}..."
    MS_TARBALL="${TEMP_DIR}/minisign-${MINISIGN_VERSION}-linux.tar.gz"
    MS_URL="https://github.com/jedisct1/minisign/releases/download/${MINISIGN_VERSION}"
    curl -fsSL "${MS_URL}/minisign-${MINISIGN_VERSION}-linux.tar.gz" -o "$MS_TARBALL" \
        || die "Could not download minisign ${MINISIGN_VERSION}."
    [ "$(sha256_of "$MS_TARBALL")" = "$MINISIGN_TARBALL_SHA256" ] \
        || die "The minisign download does not match its pinned SHA-256. Aborting."
    mkdir -p "${TEMP_DIR}/minisign"
    tar -xzf "$MS_TARBALL" -C "${TEMP_DIR}/minisign" 2> /dev/null \
        || die "Could not extract minisign ${MINISIGN_VERSION}."
    MINISIGN="${TEMP_DIR}/minisign/minisign-linux/${MINISIGN_ARCH}/minisign"
    [ -f "$MINISIGN" ] || die "minisign not found in the minisign ${MINISIGN_VERSION} archive."
    chmod +x "$MINISIGN"
}

confirm_overwrite() {
    if [ ! -f "$INSTALL_DIR/pairee" ] && [ ! -d "$CONFIG_DIR" ]; then
        return 0
    fi
    echo "${YELLOW}Warning: Pairee is already installed.${NC}"
    # /dev/tty can exist without a controlling terminal (CI, containers).
    if ! { [ -c /dev/tty ] && (: < /dev/tty) 2> /dev/null; }; then
        echo "Non-interactive shell detected. Overwriting existing installation..."
        return 0
    fi
    printf "Do you want to overwrite and update the binary? [y/N]: "
    read -r OVERWRITE < /dev/tty || OVERWRITE="n"
    case "$OVERWRITE" in
        [yY][eE][sS] | [yY]) echo "Proceeding with update..." ;;
        *)
            echo "Installation cancelled."
            exit 0
            ;;
    esac

    if [ -d "$CONFIG_DIR" ]; then
        printf "Do you want to clear old configurations, themes, and history settings? [y/N]: "
        read -r CLEAR_CONFIG < /dev/tty || CLEAR_CONFIG="n"
        case "$CLEAR_CONFIG" in
            [yY][eE][sS] | [yY])
                echo "Clearing old settings in $CONFIG_DIR..."
                rm -rf "$CONFIG_DIR"
                ;;
            *) echo "Keeping existing settings." ;;
        esac
    fi
}

latest_version() {
    echo "Fetching latest version info..."
    RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")
    VERSION=$(printf '%s\n' "$RELEASE_JSON" | grep '"tag_name":' | head -n 1 | cut -d '"' -f 4)
    [ -n "$VERSION" ] || die "Could not retrieve latest release version from GitHub API."
    case "$VERSION" in
        v[0-9]*.[0-9]*.[0-9]*) ;;
        *) die "Unexpected release tag '${VERSION}'." ;;
    esac
    case "$VERSION" in
        *[!A-Za-z0-9.-]*) die "Unexpected release tag '${VERSION}'." ;;
    esac
    echo "Latest version found: ${GREEN}${VERSION}${NC}"
}

sha256_of() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

# verify_download <file> <asset name> <url>
verify_download() {
    echo "Verifying SHA-256 checksum..."
    curl -fsSL "$3.sha256" -o "$1.sha256"
    EXPECTED_SHA=$(awk '{print $1}' "$1.sha256" | tr 'A-F' 'a-f')
    if [ -z "$EXPECTED_SHA" ] || [ "$EXPECTED_SHA" != "$(sha256_of "$1")" ]; then
        die "SHA-256 checksum mismatch for $2. Aborting."
    fi

    if [ "$SKIP_SIGNATURE" = "true" ]; then
        echo "${YELLOW}Warning: --insecure-skip-signature given; the minisign signature was not checked.${NC}"
        return 0
    fi
    # The trusted comment, covered by the signature, must name this asset.
    find_minisign
    echo "Verifying minisign signature..."
    if ! curl -fsSL "$3.minisig" -o "$1.minisig" \
        || ! SIG_OUT=$("$MINISIGN" -V -P "$MINISIGN_PUBKEY" -m "$1" -x "$1.minisig") \
        || ! printf ' %s \n' "$SIG_OUT" | tr '\t' ' ' | grep -qF " file:$2 "; then
        die "minisign signature verification failed for $2. Aborting."
    fi
}

# The archive of this machine in the latest release: its current name or,
# TRANSITION, its pre-0.11 name (only 0.11.0 carries both; drop the
# second name and LEGACY_TARGET once a later release is the latest).
choose_tarball() {
    for TARBALL in "pairee-${VERSION#v}-${PLATFORM}.tar.gz" "pairee-${VERSION}-${LEGACY_TARGET}.tar.gz"; do
        if printf '%s\n' "$RELEASE_JSON" | grep -qE "\"name\": *\"${TARBALL}\""; then
            return 0
        fi
    done
    die "Release ${VERSION} has no ${PLATFORM} archive."
}

# Sets PKG_FOLDER and BIN_SRC.
download_release() {
    choose_tarball
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${TARBALL}"

    echo "Downloading ${TARBALL}..."
    curl -fsSL "$DOWNLOAD_URL" -o "${TEMP_DIR}/${TARBALL}"
    verify_download "${TEMP_DIR}/${TARBALL}" "$TARBALL" "$DOWNLOAD_URL"

    echo "Extracting archive..."
    mkdir "${TEMP_DIR}/extract"
    tar -xzf "${TEMP_DIR}/${TARBALL}" -C "${TEMP_DIR}/extract"
    BIN_SRC=$(find "${TEMP_DIR}/extract" -type f -name pairee | head -n 1)
    [ -n "$BIN_SRC" ] || die "pairee not found in ${TARBALL}."
    PKG_FOLDER=$(dirname "$BIN_SRC")
}

# Sets PKG_FOLDER and BIN_SRC.
build_from_source() {
    echo "Cloning repository..."
    git clone --depth 1 "https://github.com/${REPO}.git" "${TEMP_DIR}/pairee_src"

    echo "Compiling Pairee (cargo build --release)..."
    (cd "${TEMP_DIR}/pairee_src" && cargo build --release) || die "Compilation failed."
    PKG_FOLDER="${TEMP_DIR}/pairee_src"
    BIN_SRC="${PKG_FOLDER}/target/release/pairee"
}

install_files() {
    echo "Installing files..."
    mkdir -p "$INSTALL_DIR" "$CONFIG_DIR/lang" "$CONFIG_DIR/help" "$CONFIG_DIR/keymaps"

    # Staged next to the target and renamed over it: a running pairee keeps
    # its old file (no "Text file busy"), and a failed copy changes nothing.
    STAGED_BIN="$INSTALL_DIR/.pairee.new.$$"
    cp "$BIN_SRC" "$STAGED_BIN"
    chmod 755 "$STAGED_BIN"
    mv -f "$STAGED_BIN" "$INSTALL_DIR/pairee"
    STAGED_BIN=""

    if [ -d "${PKG_FOLDER}/lang" ]; then
        cp -r "${PKG_FOLDER}/lang/"* "$CONFIG_DIR/lang/"
        # Clean up legacy JSON translation files
        rm -f "$CONFIG_DIR/lang/"*.json
    fi
    if [ -d "${PKG_FOLDER}/help" ]; then
        cp -r "${PKG_FOLDER}/help/"* "$CONFIG_DIR/help/"
    fi
    # Copy keybinding preset files, but skip files already edited by the user
    if [ -d "${PKG_FOLDER}/keymaps" ]; then
        for preset_file in "${PKG_FOLDER}/keymaps/"*.toml; do
            [ -f "$preset_file" ] || continue
            dest="$CONFIG_DIR/keymaps/$(basename "$preset_file")"
            if [ ! -f "$dest" ]; then
                cp "$preset_file" "$dest"
            fi
        done
    fi
}

print_summary() {
    echo "=============================="
    echo "${GREEN}Pairee version ${VERSION} installed successfully!${NC}"
    echo "Binary location: ${BLUE}${INSTALL_DIR}/pairee${NC}"
    echo "Config location: ${BLUE}${CONFIG_DIR}/${NC}"
    echo ""

    case ":$PATH:" in
        *:"$INSTALL_DIR":*) ;;
        *)
            echo "${BLUE}Note: '${INSTALL_DIR}' is not in your PATH.${NC}"
            echo "Please add it to your shell configuration (e.g. ~/.bashrc or ~/.zshrc):"
            echo "  ${GREEN}export PATH=\"\$PATH:\$HOME/.local/bin\"${NC}"
            echo ""
            ;;
    esac

    echo "Run Pairee by typing: ${GREEN}pairee${NC}"
}

main() {
    parse_args "$@"
    if [ "$UNINSTALL_MODE" = "true" ]; then
        uninstall
        return 0
    fi

    echo "${BLUE}Pairee Installer for Linux${NC}"
    echo "=============================="
    detect_platform
    check_dependencies
    if [ "$VERIFY_ONLY" != "true" ]; then
        confirm_overwrite
    fi

    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    TEMP_DIR=$(mktemp -d)

    if [ "$DEBUG_MODE" = "true" ]; then
        VERSION="debug-source"
        echo "Running in debug mode. Will compile from master branch source..."
        build_from_source
    else
        latest_version
        download_release
    fi
    if [ "$VERIFY_ONLY" = "true" ]; then
        echo "${GREEN}Pairee ${VERSION} downloaded and verified; nothing was installed (--verify-only).${NC}"
        return 0
    fi

    install_files
    print_summary
}

main "$@"
