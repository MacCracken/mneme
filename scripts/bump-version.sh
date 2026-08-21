#!/usr/bin/env bash
# bump-version.sh — set the project version.
#
# `VERSION` at the repo root is the single source of truth: `cyrius.cyml` reads
# it via `version = "${file:VERSION}"`, and CI reads the toolchain pin from
# `cyrius.cyml [package].cyrius`. So a release bump is a one-file edit — there is
# nothing to keep in sync, which is the point.
#
# (Before v1.1.2 this script sed-ed `Cargo.toml`, left over from the Rust
# original. No Cargo.toml has existed since the Cyrius port, and with
# `set -euo pipefail` the sed made this script fail outright.)
#
# Usage:
#   ./scripts/bump-version.sh           # show the current version
#   ./scripts/bump-version.sh 1.2.0     # set an explicit version
#   ./scripts/bump-version.sh patch     # 1.1.1 -> 1.1.2
#   ./scripts/bump-version.sh minor     # 1.1.1 -> 1.2.0
#   ./scripts/bump-version.sh major     # 1.1.1 -> 2.0.0

set -euo pipefail
cd "$(dirname "$0")/.."

VERSION_FILE="VERSION"
OLD_VERSION="$(tr -d '[:space:]' < "$VERSION_FILE")"

if [[ $# -eq 0 ]]; then
    echo "Current version: $OLD_VERSION"
    exit 0
fi

if [[ ! "$OLD_VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    echo "error: VERSION ('$OLD_VERSION') is not MAJOR.MINOR.PATCH" >&2
    exit 1
fi
major="${BASH_REMATCH[1]}"; minor="${BASH_REMATCH[2]}"; patch="${BASH_REMATCH[3]}"

case "$1" in
    major) NEW_VERSION="$((major + 1)).0.0" ;;
    minor) NEW_VERSION="${major}.$((minor + 1)).0" ;;
    patch) NEW_VERSION="${major}.${minor}.$((patch + 1))" ;;
    *)
        if [[ ! "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "error: '$1' is not MAJOR.MINOR.PATCH (or major|minor|patch)" >&2
            exit 1
        fi
        NEW_VERSION="$1"
        ;;
esac

echo "$NEW_VERSION" > "$VERSION_FILE"
echo "Bumped version: $OLD_VERSION -> $NEW_VERSION"
echo
echo "Still to do by hand (this script deliberately does not touch prose):"
echo "  - CHANGELOG.md                  add the [$NEW_VERSION] entry"
echo "  - docs/development/state.md     refresh Version + Toolchain + Tests"
