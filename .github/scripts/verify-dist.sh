#!/usr/bin/env bash
# Smoke-tests the packages in dist/ before they are uploaded (release.yml).
#
#   TARGET   target triple of the build
#   VERSION  release tag (vX.Y.Z)
#   RUNNABLE "true" when the runner can execute binaries of TARGET
#
# Every package must carry VERSION; where the runner can execute the binary,
# the copy inside each package must run and report it (`pairee --version`).
# The file names come from `cargo xtask asset-names`, as in package.sh.
set -euo pipefail

: "${TARGET:?}" "${VERSION:?}"
NUM="${VERSION#v}"
# Sets STEM, ARCHIVE, LEGACY, SETUP, MSIX and ARCH.
eval "$(cargo xtask asset-names "$TARGET" "$VERSION")"
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
    extract "dist/${ARCHIVE}" "$WORK/zip"
    ROOT="$WORK/zip/${STEM}"
    EXE="pairee.exe"
    ;;
  *)
    mkdir -p "$WORK/tar"
    tar -xzf "dist/${ARCHIVE}" -C "$WORK/tar" || fail "cannot extract ${ARCHIVE}"
    ROOT="$WORK/tar/${STEM}"
    EXE="pairee"
    ;;
esac
for item in "$EXE" lang help README.md LICENSE; do
  [ -e "$ROOT/$item" ] || fail "archive is missing ${item}"
done
ok "archive contents"
check_binary "$ROOT/$EXE" archive
# TRANSITION (only 0.11.0): the copy under the pre-0.11 name.
if [ -n "$LEGACY" ]; then
  cmp -s "dist/${ARCHIVE}" "dist/${LEGACY}" || fail "${LEGACY} is not a copy of ${ARCHIVE}"
  ok "legacy copy ${LEGACY}"
fi

case "$TARGET" in
  *windows*)
    SETUP="dist/${SETUP}"
    [ -s "$SETUP" ] || fail "missing $(basename "$SETUP")"
    if [ "${RUNNABLE:-false}" = true ]; then
      # Silent install with a deadline: a setup that waits for input must not
      # hold the job until the 6-hour limit. Exit 124 = timed out.
      SETUP_W="$(cygpath -w "$SETUP")"
      DIR="$(cygpath -w "$WORK/installed")"
      LOG="$(cygpath -w "$WORK/setup.log")"
      status=0
      powershell -NoProfile -Command "
        \$p = Start-Process -FilePath '${SETUP_W}' -PassThru -ArgumentList \
          '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-','/NOICONS','/DIR=\"${DIR}\"','/LOG=\"${LOG}\"';
        if (-not \$p.WaitForExit(180000)) { Get-Process | Where-Object { \$_.Path -like '*\\Temp\\is-*' } | Stop-Process -Force; \$p.Kill(); exit 124 }
        exit \$p.ExitCode" || status=$?
      if [ "$status" -ne 0 ]; then
        cat "$WORK/setup.log" 2> /dev/null || true
        [ "$status" -eq 124 ] && fail "the installer did not finish within 3 minutes (log above)"
        fail "the installer exited with ${status} (log above)"
      fi
      check_binary "$WORK/installed/pairee.exe" installer
    fi

    MSIX="dist/${MSIX}"
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
