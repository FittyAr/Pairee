#!/usr/bin/env bash
# Signs files with the Pairee release key (minisign) and checks each
# signature against the public key embedded in the updater. Used by
# release.yml for the release assets and by the plugin-registry branch's
# sign-registry.yml for the registry files.
#
#   sign.sh [file...]    files to sign, relative to SIGN_DIR; none: every
#                        file directly in SIGN_DIR except .sha256 / .minisig
#
#   MINISIGN_SECRET_KEY  contents of the secret key file (required)
#   MINISIGN_PASSWORD    its password, when it has one
#   GITHUB_REF_NAME      release tag (or branch), recorded in the trusted comment
#   SIGN_DIR             folder of the files (default: dist)
#   SIGNATURE_RS         the updater source holding RELEASE_PUBLIC_KEY
#                        (default: src/update/signature.rs)
#
# The trusted comment names the file by its path relative to SIGN_DIR
# (`file:<path>`), so a signature cannot be reused for another file. The
# updater, the installers and the plugin registry client require it.
set -euo pipefail

: "${GITHUB_REF_NAME:?}"
if [ -z "${MINISIGN_SECRET_KEY:-}" ]; then
  echo "::error::Repository secret MINISIGN_SECRET_KEY is not set; refusing to publish unsigned files."
  exit 1
fi

# The single source of the public key; tests/release_key.rs keeps the
# install scripts in line with it.
SIGNATURE_RS="${SIGNATURE_RS:-src/update/signature.rs}"
PUBKEY=$(grep -oP 'RELEASE_PUBLIC_KEY: &str = "\K[^"]+' "$SIGNATURE_RS")
[ -n "$PUBKEY" ] || { echo "::error::no RELEASE_PUBLIC_KEY in $SIGNATURE_RS"; exit 1; }

umask 077
KEY_FILE="${RUNNER_TEMP:-/tmp}/pairee-release.key"
trap 'rm -f "$KEY_FILE"' EXIT
printf '%s\n' "$MINISIGN_SECRET_KEY" > "$KEY_FILE"

# sign <path>: writes <path>.minisig and checks it against the public key
# embedded in the binary (the secret must match it).
sign() {
  local comment="file:$1 version:${GITHUB_REF_NAME}"
  if [ -n "${MINISIGN_PASSWORD:-}" ]; then
    printf '%s\n' "$MINISIGN_PASSWORD" | minisign -S -s "$KEY_FILE" -m "$1" -t "$comment"
  else
    minisign -S -s "$KEY_FILE" -m "$1" -t "$comment" < /dev/null
  fi
  test -s "$1.minisig"
  minisign -V -q -P "$PUBKEY" -m "$1"
}

cd "${SIGN_DIR:-dist}"
if [ $# -eq 0 ]; then
  for f in *; do
    [ -f "$f" ] || continue
    case "$f" in *.sha256 | *.minisig) continue ;; esac
    sign "$f"
  done
  ls -l
else
  for f in "$@"; do
    sign "$f"
  done
fi
