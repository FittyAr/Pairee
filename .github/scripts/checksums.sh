#!/usr/bin/env bash
# Writes <file>.sha256 (the bare hex digest) next to every file in dist/
# (release.yml). The updater and the install scripts read these files.
set -euo pipefail

cd dist
for f in *; do
  [ -f "$f" ] || continue
  case "$f" in *.sha256 | *.minisig) continue ;; esac
  if command -v sha256sum > /dev/null; then
    sha256sum "$f" | awk '{print $1}' > "${f}.sha256"
  else
    shasum -a 256 "$f" | awk '{print $1}' > "${f}.sha256"
  fi
done
