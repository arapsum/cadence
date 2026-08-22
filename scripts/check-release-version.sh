#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)

if (($# != 1)); then
    printf 'Usage: scripts/check-release-version.sh TAG\n' >&2
    exit 2
fi

tag=$1
version=${tag#v}
manifest_version=$(sed -nE 's/^version = "([0-9][^"]*)"/\1/p' "${REPO_ROOT}/Cargo.toml" | head -n 1)

if [[ "$tag" != "v${manifest_version}" ]]; then
    printf 'error: tag %s does not match Cargo.toml version %s\n' "$tag" "$manifest_version" >&2
    exit 1
fi
if ! grep -Fq "## [${version}]" "${REPO_ROOT}/CHANGELOG.md"; then
    printf 'error: CHANGELOG.md has no release section for %s\n' "$version" >&2
    exit 1
fi

printf 'Release tag %s matches version %s.\n' "$tag" "$manifest_version"
