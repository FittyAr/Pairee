#!/usr/bin/env bash
# Packages the release build of one target into dist/ (release.yml).
#
#   TARGET   target triple of the build
#   VERSION  release tag (vX.Y.Z)
#
# Windows: the zip, the Inno Setup installer and the MSIX (package-msix.ps1).
# Elsewhere: the tar.gz; the x86_64 musl build also makes the .deb and .rpm.
# The file names come from `cargo xtask asset-names` (src/update/assets.rs).
set -euo pipefail

: "${TARGET:?}" "${VERSION:?}"
NUM="${VERSION#v}"
# Sets STEM, ARCHIVE, LEGACY, SETUP, MSIX and ARCH.
eval "$(cargo xtask asset-names "$TARGET" "$VERSION")"

mkdir -p "$STEM" dist
cp -r lang help README.md LICENSE "$STEM/"

case "$TARGET" in
  *windows*)
    cp "target/${TARGET}/release/pairee.exe" "$STEM/"
    powershell -NoProfile -Command \
      "Compress-Archive -Path '${STEM}' -DestinationPath 'dist/${ARCHIVE}' -Force"

    MSYS_NO_PATHCONV=1 "C:\\Program Files (x86)\\Inno Setup 6\\iscc.exe" \
      /DAppVersion="$NUM" /DSourceDir="target\\${TARGET}\\release" \
      /O"target\\release" /F"${SETUP%.exe}" installer.iss
    mv "target/release/${SETUP}" dist/

    powershell -NoProfile -ExecutionPolicy Bypass -File .github/scripts/package-msix.ps1 \
      -Exe "target/${TARGET}/release/pairee.exe" -Arch "$ARCH" -Version "$NUM" \
      -OutFile "dist/${MSIX}"
    ;;
  *)
    cp "target/${TARGET}/release/pairee" "$STEM/"
    tar -czf "dist/${ARCHIVE}" "$STEM"

    if [ "$TARGET" = x86_64-unknown-linux-musl ]; then
      cargo deb --target "$TARGET" --no-build
      mv "target/${TARGET}/debian/"*.deb dist/
      cargo generate-rpm --target "$TARGET"
      mv "target/${TARGET}/generate-rpm/"*.rpm dist/
    fi
    ;;
esac

# TRANSITION (until 0.12.0, see LEGACY_NAMES_UNTIL in xtask/src/assets.rs):
# the archive also goes out under its pre-0.11 name, which the updater of
# 0.10 and older looks for. checksums.sh and sign.sh give the copy its own
# .sha256 and .minisig (signed for that name).
if [ -n "$LEGACY" ]; then
  cp "dist/${ARCHIVE}" "dist/${LEGACY}"
fi
ls -l dist
