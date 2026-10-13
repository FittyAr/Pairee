#!/usr/bin/env bash
# Signs every release asset in dist/ with the Pairee release key (minisign)
# and checks each signature against the public key embedded in the updater
# (release.yml).
#
#   MINISIGN_SECRET_KEY  contents of the secret key file (required)
#   MINISIGN_PASSWORD    its password, when it has one
#   GITHUB_REF_NAME      release tag, recorded in the trusted comment
#
# The trusted comment names the file (`file:<asset>`), so a signature cannot
# be reused for another asset. The updater and installers require it.
set -euo pipefail

: "${GITHUB_REF_NAME:?}"
if [ -z "${MINISIGN_SECRET_KEY:-}" ]; then
  echo "::error::Repository secret MINISIGN_SECRET_KEY is not set; refusing to publish unsigned assets."
  exit 1
fi

# The single source of the public key; tests/release_key.rs keeps the
# install scripts in line with it.
SIGNATURE_RS="src/update/signature.rs"
PUBKEY=$(grep -oP 'RELEASE_PUBLIC_KEY: &str = "\K[^"]+' "$SIGNATURE_RS")
[ -n "$PUBKEY" ] || { echo "::error::no RELEASE_PUBLIC_KEY in $SIGNATURE_RS"; exit 1; }

umask 077
KEY_FILE="${RUNNER_TEMP:-/tmp}/pairee-release.key"
trap 'rm -f "$KEY_FILE"' EXIT
printf '%s\n' "$MINISIGN_SECRET_KEY" > "$KEY_FILE"

cd dist
for f in *; do
  [ -f "$f" ] || continue
  case "$f" in *.sha256 | *.minisig) continue ;; esac
  COMMENT="file:${f} version:${GITHUB_REF_NAME}"
  if [ -n "${MINISIGN_PASSWORD:-}" ]; then
    printf '%s\n' "$MINISIGN_PASSWORD" | minisign -S -s "$KEY_FILE" -m "$f" -t "$COMMENT"
  else
    minisign -S -s "$KEY_FILE" -m "$f" -t "$COMMENT" < /dev/null
  fi
  test -s "${f}.minisig"
done

# The secret must match the public key embedded in the binary.
for f in *.minisig; do
  minisign -V -q -P "$PUBKEY" -m "${f%.minisig}"
done
ls -l
