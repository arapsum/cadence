#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)
DIST_DIR="${REPO_ROOT}/target/dist"
SKIP_BUILD=false
VERSION=''

usage() {
    cat <<'EOF'
Usage: scripts/package-linux.sh [OPTIONS] [VERSION]

Build a reproducible Cadence .deb for Ubuntu 26.04 x86_64.

Options:
  --skip-build  Package the existing target/release/cadence binary.
  -h, --help    Show this help message.
EOF
}

while (($# > 0)); do
    case "$1" in
        --skip-build)
            SKIP_BUILD=true
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --*)
            printf 'error: unknown option: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
        *)
            if [[ -n "$VERSION" ]]; then
                printf 'error: more than one version was provided\n' >&2
                exit 2
            fi
            VERSION=$1
            ;;
    esac
    shift
done

if [[ -z "$VERSION" ]]; then
    VERSION=$(sed -nE 's/^version = "([0-9][^"]*)"/\1/p' "${REPO_ROOT}/Cargo.toml" | head -n 1)
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
    printf 'error: invalid package version: %s\n' "$VERSION" >&2
    exit 2
fi

case "$(uname -m)" in
    x86_64) ARCH=amd64 ;;
    *)
        printf 'error: this release only supports x86_64 (found %s)\n' "$(uname -m)" >&2
        exit 1
        ;;
esac

for command in cargo dpkg-deb install sha256sum sed awk find git; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf 'error: required command is not installed: %s\n' "$command" >&2
        exit 1
    fi
done

BINARY="${REPO_ROOT}/target/release/cadence"
if [[ "$SKIP_BUILD" != true ]]; then
    build_commit=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD 2>/dev/null || printf 'development')
    CADENCE_BUILD_COMMIT="$build_commit" cargo build --locked --release -p cadence-desktop
fi
if [[ ! -x "$BINARY" ]]; then
    printf 'error: release binary is missing or not executable: %s\n' "$BINARY" >&2
    exit 1
fi

mkdir -p "$DIST_DIR"
stage=$(mktemp -d "${TMPDIR:-/tmp}/cadence-deb.XXXXXX")
cleanup() {
    rm -rf "$stage"
}
trap cleanup EXIT

install -d \
    "$stage/DEBIAN" \
    "$stage/usr/bin" \
    "$stage/usr/share/applications" \
    "$stage/usr/share/icons/hicolor/scalable/apps" \
    "$stage/usr/share/metainfo" \
    "$stage/usr/share/doc/cadence"
install -m 0755 "$BINARY" "$stage/usr/bin/cadence"
install -m 0644 \
    "$REPO_ROOT/packaging/linux/io.github.arapsum.Cadence.desktop" \
    "$stage/usr/share/applications/io.github.arapsum.Cadence.desktop"
install -m 0644 \
    "$REPO_ROOT/packaging/linux/io.github.arapsum.Cadence.metainfo.xml" \
    "$stage/usr/share/metainfo/io.github.arapsum.Cadence.metainfo.xml"
install -m 0644 \
    "$REPO_ROOT/crates/ui/assets/cadence-icon.svg" \
    "$stage/usr/share/icons/hicolor/scalable/apps/io.github.arapsum.Cadence.svg"
for document in README.md LICENSE CHANGELOG.md docs/USER_GUIDE.md packaging/linux/copyright; do
    install -m 0644 "$REPO_ROOT/$document" "$stage/usr/share/doc/cadence/$(basename "$document")"
done

runtime_dependencies='libvulkan1, libwayland-client0, libwayland-cursor0, libwayland-egl1, libfontconfig1, libxkbcommon0, libxcb-xkb1, libxau6, libxdmcp6'
dependencies="libc6, libgcc-s1, libstdc++6, ${runtime_dependencies}"
if command -v dpkg-shlibdeps >/dev/null 2>&1; then
    dependency_workspace="$stage/.dependency-work"
    install -d "$dependency_workspace/debian"
    printf 'Source: cadence\nSection: utils\nPriority: optional\nMaintainer: Kibet arap Sum <kibetarapsum@gmail.com>\n\nPackage: cadence\nArchitecture: any\nDepends: \nDescription: Cadence runtime dependency probe\n Cadence runtime dependency probe.\n' \
        > "$dependency_workspace/debian/control"
    shlib_output=$(
        cd "$dependency_workspace"
        dpkg-shlibdeps -O --package=cadence -e "$stage/usr/bin/cadence" 2>/dev/null || true
    )
    generated_dependencies=$(printf '%s\n' "$shlib_output" | sed -n 's/^shlibs:Depends=//p')
    rm -rf "$dependency_workspace"
    if [[ -n "$generated_dependencies" ]]; then
        dependencies="$generated_dependencies, libstdc++6, ${runtime_dependencies}"
    fi
fi

printf 'Package: cadence\nVersion: %s\nArchitecture: %s\nSection: utils\nPriority: optional\nMaintainer: Kibet arap Sum <kibetarapsum@gmail.com>\nDepends: %s\nDescription: local-first desktop timetable\n Cadence helps plan a day and understand a week.\n It stores timetable data locally and works offline.\n' \
    "$VERSION" "$ARCH" "$dependencies" > "$stage/DEBIAN/control"

source_date_epoch=$(git -C "$REPO_ROOT" show -s --format=%ct HEAD 2>/dev/null || date +%s)
find "$stage" -type d -exec chmod 0755 {} +
find "$stage" -print0 | xargs -0 touch --date "@${source_date_epoch}"

output="${DIST_DIR}/cadence_${VERSION}_${ARCH}.deb"
rm -f "$output"
SOURCE_DATE_EPOCH="$source_date_epoch" dpkg-deb --build --root-owner-group "$stage" "$output" >/dev/null

(cd "$DIST_DIR" && sha256sum "$(basename "$output")" > SHA256SUMS)
printf 'Created %s\n' "$output"
printf 'Checksums: %s\n' "$DIST_DIR/SHA256SUMS"
