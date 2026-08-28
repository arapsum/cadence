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

if [[ "$manifest_version" != "$version" ]]; then
    printf 'error: Cargo.toml version %s does not match release version %s\n' "$manifest_version" "$version" >&2
    exit 1
fi
if ! grep -Eq "^## \\[${version}\\]([[:space:]]|$)" "${REPO_ROOT}/CHANGELOG.md"; then
    printf 'error: CHANGELOG.md has no release heading for %s\n' "$version" >&2
    exit 1
fi

appstream_release_version=$(
    sed -nE '/<releases>/,/<\/releases>/ {
        /<release[[:space:]]/ {
            s/.*version="([^"]+)".*/\1/p
            q
        }
    }' "${REPO_ROOT}/packaging/linux/io.github.arapsum.Cadence.metainfo.xml"
)
if [[ "$appstream_release_version" != "$version" ]]; then
    printf 'error: packaging/linux/io.github.arapsum.Cadence.metainfo.xml first release version %s does not match release version %s\n' \
        "${appstream_release_version:-<missing>}" "$version" >&2
    exit 1
fi

if [[ "$tag" != "v${version}" ]]; then
    printf 'error: release tag %s must use the v<version> form for %s\n' "$tag" "$version" >&2
    exit 1
fi

printf 'Release tag %s matches version %s.\n' "$tag" "$version"

