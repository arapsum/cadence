#!/usr/bin/env bash
set -euo pipefail

if (($# != 1)) || [[ "$1" == '-h' || "$1" == '--help' ]]; then
    printf 'Usage: scripts/verify-linux-install.sh PATH_TO_DEB\n' >&2
    exit $(( $# == 1 ? 0 : 2 ))
fi

package=$(realpath "$1")
if [[ ! -f "$package" ]]; then
    printf 'error: package does not exist: %s\n' "$package" >&2
    exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
    printf 'error: Docker is required for clean Ubuntu installation checks\n' >&2
    exit 1
fi

docker run --rm --pull=missing \
    --volume "$package:/tmp/cadence.deb:ro" \
    ubuntu:26.04 \
    bash -ceu '
        export DEBIAN_FRONTEND=noninteractive
        apt-get update
        apt-get install --yes --no-install-recommends /tmp/cadence.deb dpkg-dev

        # Build a lower-version copy so apt exercises a real upgrade path.
        dpkg-deb --extract /tmp/cadence.deb /tmp/cadence-old
        dpkg-deb --control /tmp/cadence.deb /tmp/cadence-old/DEBIAN
        sed -i "s/^Version: .*/Version: 0.0.0/" /tmp/cadence-old/DEBIAN/control
        dpkg-deb --build --root-owner-group /tmp/cadence-old /tmp/cadence-old.deb >/dev/null
        apt-get install --yes --allow-downgrades /tmp/cadence-old.deb
        apt-get install --yes /tmp/cadence.deb
        cadence --version

        data_dir=/root/.local/share/cadence
        mkdir -p "$data_dir"
        printf "retained across package removal\n" > "$data_dir/install-check.txt"
        cp -a "$data_dir" /tmp/cadence-backup
        apt-get remove --yes cadence
        test -f "$data_dir/install-check.txt"
        test -f /tmp/cadence-backup/install-check.txt
        printf "Clean install, upgrade, uninstall, and data-retention checks passed.\n"
    '
