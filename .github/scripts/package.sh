#!/usr/bin/env bash
# Packages the release build of one target into dist/ (release.yml).
#
#   TARGET   target triple of the build
#   VERSION  release tag (vX.Y.Z)
#
# Windows: the zip, the Inno Setup installer and the MSIX (package-msix.ps1).
# Elsewhere: the tar.gz; the x86_64 musl build also makes the .deb and .rpm.
set -euo pipefail

: "${TARGET:?}" "${VERSION:?}"
NUM="${VERSION#v}"
PKG="pairee-${VERSION}-${TARGET}"

mkdir -p "$PKG" dist
cp -r lang help README.md LICENSE "$PKG/"

case "$TARGET" in
  *windows*)
    ARCH=x64
    [[ "$TARGET" == aarch64-* ]] && ARCH=arm64

    cp "target/${TARGET}/release/pairee.exe" "$PKG/"
    powershell -NoProfile -Command \
      "Compress-Archive -Path '${PKG}' -DestinationPath 'dist/${PKG}.zip' -Force"

    MSYS_NO_PATHCONV=1 "C:\\Program Files (x86)\\Inno Setup 6\\iscc.exe" \
      /DAppVersion="$NUM" /DSourceDir="target\\${TARGET}\\release" \
      /O"target\\release" /F"pairee-setup-${NUM}-${ARCH}" installer.iss
    mv "target/release/pairee-setup-${NUM}-${ARCH}.exe" dist/

    powershell -NoProfile -ExecutionPolicy Bypass -File .github/scripts/package-msix.ps1 \
      -Target "$TARGET" -Arch "$ARCH" -Version "$NUM"
    ;;
  *)
    cp "target/${TARGET}/release/pairee" "$PKG/"
    tar -czf "dist/${PKG}.tar.gz" "$PKG"

    if [ "$TARGET" = x86_64-unknown-linux-musl ]; then
      cargo deb --target "$TARGET" --no-build
      mv "target/${TARGET}/debian/"*.deb dist/
      cargo generate-rpm --target "$TARGET"
      mv "target/${TARGET}/generate-rpm/"*.rpm dist/
    fi
    ;;
esac
ls -l dist
