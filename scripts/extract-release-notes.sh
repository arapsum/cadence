#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)

if (($# != 1)); then
    printf 'Usage: scripts/extract-release-notes.sh VERSION\n' >&2
    exit 2
fi

version=$1
awk -v version="$version" '
    $0 ~ "^## \\[" version "\\]" { found = 1; next }
    found && /^## / { exit }
    found { print }
    END {
        if (!found) {
            exit 1
        }
    }
' "${REPO_ROOT}/CHANGELOG.md"
