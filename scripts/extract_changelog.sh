#!/usr/bin/env bash
# extract_changelog.sh
# Extracts a specific version section from CHANGELOG.md and outputs it to stdout.
# Usage: ./scripts/extract_changelog.sh v0.5.1
#        ./scripts/extract_changelog.sh Unreleased
#
# The output is suitable for use as a GitHub Release body.

set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>" >&2
    echo "  Examples: $0 v0.5.1" >&2
    echo "            $0 Unreleased" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Normalise version and determine target file
if [[ "${VERSION,,}" == "unreleased" ]]; then
    TARGET_FILE="$SCRIPT_DIR/../docs/UNRELEASED.md"
    SECTION_HEADER="## [Unreleased]"
else
    TARGET_FILE="$SCRIPT_DIR/../docs/CHANGELOG.md"
    if [[ "$VERSION" == v* ]]; then
        SECTION_HEADER="## [$VERSION]"
    else
        SECTION_HEADER="## [v$VERSION]"
    fi
fi

if [[ ! -f "$TARGET_FILE" ]]; then
    echo "Error: File not found at $TARGET_FILE" >&2
    exit 1
fi

# Extract lines between this section header and the next ## [ header
OUTPUT=$(awk -v header="$SECTION_HEADER" '
    BEGIN { in_section = 0 }
    {
        # Match section header (exact or with date suffix e.g. "## [v0.5.1] - 2026-06-25")
        if ($0 == header || index($0, header " ") == 1 || index($0, header "-") == 1) {
            in_section = 1
            next
        }
        if (in_section && /^## \[/) {
            exit
        }
        if (in_section) {
            print
        }
    }
' "$TARGET_FILE" | sed -e '/./,$!d' -e 's/[[:space:]]*$//')

if [[ -z "$OUTPUT" ]]; then
    echo "Error: Section '$SECTION_HEADER' not found or empty in $(basename "$TARGET_FILE")" >&2
    exit 1
fi

echo "$OUTPUT"
