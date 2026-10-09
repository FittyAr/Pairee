#!/usr/bin/env bash
# Smoke-tests the packages in dist/ before they are uploaded (release.yml).
#
#   TARGET   target triple of the build
#   VERSION  release tag (vX.Y.Z)
#   RUNNABLE "true" when the runner can execute binaries of TARGET
#
# Every package must carry VERSION; where the runner can execute the binary,
# the copy inside each package must run and report it (`pairee --version`).
set -euo pipefail

: "${TARGET:?}" "${VERSION:?}"
NUM="${VERSION#v}"
PKG="pairee-${VERSION}-${TARGET}"
WORK="$(mktemp -d)"

fail() { echo "::error::${TARGET}: $*"; exit 1; }
ok() { echo "ok: $*"; }

# check_binary <path> <label>
check_binary() {
  [ "${RUNNABLE:-false}" = true ] || return 0
  local out
  out="$("$1" --version)" || fail "$2: '$1 --version' failed"
  [ "$out" = "pairee ${NUM}" ] || fail "$2: reports '${out}', expected 'pairee ${NUM}'"
  ok "$2 runs and reports ${NUM}"
}

# extract <archive> <dir> [member]: zip-family archives (zip, msix)
extract() {
  mkdir -p "$2"
  7z x -y -bd -o"$2" "$1" ${3:+"$3"} > /dev/null || fail "cannot extract $1"
}

case "$TARGET" in
  *windows*)
    extract "dist/${PKG}.zip" "$WORK/zip"
    ROOT="$WORK/zip/${PKG}"
    EXE="pairee.exe"
    ;;
  *)
    mkdir -p "$WORK/tar"
    tar -xzf "dist/${PKG}.tar.gz" -C "$WORK/tar" || fail "cannot extract ${PKG}.tar.gz"
    ROOT="$WORK/tar/${PKG}"
    EXE="pairee"
    ;;
esac
for item in "$EXE" lang help README.md LICENSE; do
  [ -e "$ROOT/$item" ] || fail "archive is missing ${item}"
done
ok "archive contents"
check_binary "$ROOT/$EXE" archive

case "$TARGET" in
  *windows*)
    ARCH=x64
    [[ "$TARGET" == aarch64-* ]] && ARCH=arm64

    SETUP="dist/pairee-setup-${NUM}-${ARCH}.exe"
    [ -s "$SETUP" ] || fail "missing $(basename "$SETUP")"
    if [ "${RUNNABLE:-false}" = true ]; then
      DIR="$(cygpath -w "$WORK/installed")"
      "$SETUP" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CURRENTUSER "/DIR=${DIR}" ||
        fail "the installer failed"
      check_binary "$WORK/installed/pairee.exe" installer
    fi

    MSIX="dist/pairee-${NUM}-${ARCH}.msix"
    [ -s "$MSIX" ] || fail "missing $(basename "$MSIX")"
    extract "$MSIX" "$WORK/msix" AppxManifest.xml
    grep -q "Version=\"${NUM}.0\"" "$WORK/msix/AppxManifest.xml" || fail "MSIX version is not ${NUM}.0"
    grep -q "ProcessorArchitecture=\"${ARCH}\"" "$WORK/msix/AppxManifest.xml" ||
      fail "MSIX architecture is not ${ARCH}"
    ok "MSIX ${NUM}.0 (${ARCH})"
    ;;
  x86_64-unknown-linux-musl)
    DEB="$(ls dist/*.deb)"
    DEB_VERSION="$(dpkg-deb -f "$DEB" Version)"
    [[ "$DEB_VERSION" == "$NUM" || "$DEB_VERSION" == "$NUM"-* ]] ||
      fail "deb version is ${DEB_VERSION}"
    dpkg-deb -x "$DEB" "$WORK/deb"
    check_binary "$WORK/deb/usr/bin/pairee" deb

    RPM="$(ls dist/*.rpm)"
    RPM_VERSION="$(rpm -qp --queryformat '%{VERSION}' "$RPM")"
    [ "$RPM_VERSION" = "$NUM" ] || fail "rpm version is ${RPM_VERSION}"
    ok "rpm ${RPM_VERSION}"
    ;;
esac
