#!/usr/bin/env bash
# bump-version.sh — Update all version references consistently.
#
# Usage:
#   ./scripts/bump-version.sh              # set version from VERSION file
#   ./scripts/bump-version.sh 2026.3.18    # set specific version
#   ./scripts/bump-version.sh patch        # bump to YYYY.M.D-N (increment N)
#   ./scripts/bump-version.sh today        # set to today's date
#
# Version format: YYYY.M.D or YYYY.M.D-N for patches

set -euo pipefail
cd "$(dirname "$0")/.."

VERSION_FILE="VERSION"
OLD_VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')

if [[ $# -ge 1 ]]; then
    if [[ "$1" == "patch" ]]; then
        if [[ "$OLD_VERSION" =~ ^([0-9]+\.[0-9]+\.[0-9]+)-([0-9]+)$ ]]; then
            base="${BASH_REMATCH[1]}"
            n="${BASH_REMATCH[2]}"
            NEW_VERSION="${base}-$((n + 1))"
        else
            NEW_VERSION="${OLD_VERSION}-1"
        fi
    elif [[ "$1" == "today" ]]; then
        year=$(date +%Y)
        month=$(date +%-m)
        day=$(date +%-d)
        NEW_VERSION="${year}.${month}.${day}"
    else
        NEW_VERSION="$1"
    fi
    echo "$NEW_VERSION" > "$VERSION_FILE"
else
    NEW_VERSION="$OLD_VERSION"
fi

VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')
echo "Bumping version: $OLD_VERSION → $VERSION"

# Update workspace version in root Cargo.toml
sed -i "s/^version = \"$OLD_VERSION\"/version = \"$VERSION\"/" Cargo.toml

# Also update any Cargo.toml files that have hardcoded versions (safety net)
find . -name "Cargo.toml" -not -path "*/target/*" -exec \
    sed -i "s/version = \"$OLD_VERSION\"/version = \"$VERSION\"/g" {} \;

echo ""
echo "Updated:"
echo "  VERSION              → $VERSION"
echo "  Cargo.toml (all)     → $VERSION"
echo ""
echo "Don't forget to update CHANGELOG.md!"

RELEASE_NAME=$(echo "$VERSION" | sed 's/\.//g; s/-//g')
echo "Release filename stem: mneme-${RELEASE_NAME}"
