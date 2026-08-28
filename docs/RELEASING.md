# Cadence release procedure

Cadence publishes Ubuntu 26.04 x86_64 `.deb` artifacts from annotated version
tags. Releases are created as drafts so a maintainer can install the exact
artifact and complete the host Wayland acceptance gate before publishing it.

## Prepare a release

1. Update the workspace version in the root `Cargo.toml`, the three first-party
   package entries (`cadence-core`, `cadence-ui`, and `cadence-desktop`) in
   `Cargo.lock`, the matching `CHANGELOG.md` section, and the top AppStream
   release entry in `packaging/linux/io.github.arapsum.Cadence.metainfo.xml`.
2. Run the repository checks before opening the release pull request:

   ```sh
   cargo fmt --all -- --check
   cargo +stable clippy --workspace --locked --all-targets --all-features -- -D warnings \
     -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms
   cargo test --workspace --locked --all-targets --all-features
   scripts/check-release-version.sh v<version>
   ```

3. Commit the preparation on a `release/v<version>` branch, open a pull request
   to protected `main`, and wait for both required CI checks.
4. Merge the pull request and wait for the `main` CI run to pass. The release
   tag MUST be created only after this exact merged commit is the tip of
   `origin/main`.
5. Update local `main`, verify the declarations, and create an annotated tag
   from that exact commit:

   ```sh
   git switch main
   git pull --ff-only origin main
   scripts/check-release-version.sh v<version>
   test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
   git tag -a v<version> -m "Cadence <version>"
   test "$(git rev-parse 'v<version>^{}')" = "$(git rev-parse HEAD)"
   git push origin v<version>
   ```

The tag workflow rejects a tag whose commit is behind or otherwise different
from the fetched `origin/main` tip. This prevents a release artifact from being
built from an identity other than the exact merged release commit.

## Automated release publisher

`.github/workflows/release.yml` is the release publisher. It keeps the version
script as the first release gate, fetches `origin/main`, requires the tag
commit to equal that tip, and then runs the CI-equivalent build:

```sh
CADENCE_BUILD_COMMIT="${GITHUB_SHA::12}" cargo build --locked --release -p cadence-desktop
scripts/package-linux.sh --skip-build "${GITHUB_REF_NAME#v}"
scripts/validate-linux-package.sh "target/dist/cadence_${GITHUB_REF_NAME#v}_amd64.deb"
scripts/verify-linux-install.sh "target/dist/cadence_${GITHUB_REF_NAME#v}_amd64.deb"
```

`scripts/package-linux.sh` is the local package builder; there is no
`scripts/release-linux.sh`. The checked-out 12-character commit is embedded in
the binary through `CADENCE_BUILD_COMMIT`, so the package MUST be built from
the exact annotated tag when reproducing a published release identity.

For local reproduction, check out and verify the tag first, then build before
using `--skip-build`:

```sh
git checkout v<version>
test "$(git rev-parse 'v<version>^{}')" = "$(git rev-parse HEAD)"
scripts/check-release-version.sh v<version>
CADENCE_BUILD_COMMIT="$(git rev-parse --short=12 HEAD)" cargo build --locked --release -p cadence-desktop
scripts/package-linux.sh --skip-build <version>
scripts/validate-linux-package.sh target/dist/cadence_<version>_amd64.deb
scripts/verify-linux-install.sh target/dist/cadence_<version>_amd64.deb
```

This reproduces the tagged build identity, including the commit shown by
`cadence --version`; it does not promise byte-for-byte equivalence with the
published package.

## Published-package and host acceptance gate

The published package path is:

1. Download `cadence_<version>_amd64.deb` and `SHA256SUMS` from the draft or
   published GitHub release.
2. Verify the artifact:

   ```sh
   sha256sum -c SHA256SUMS
   ```

3. For a clean install, remove any old package first, then install the released
   `.deb`:

   ```sh
   sudo apt remove --yes cadence
   sudo apt install --yes ./cadence_<version>_amd64.deb
   ```

Only the Ubuntu amd64 `.deb` is published and supported until an RPM artifact
and its release pipeline exist.

`scripts/validate-linux-package.sh` validates package metadata and contents.
`scripts/verify-linux-install.sh` covers Debian install, upgrade, uninstall,
and data retention only. The container verifier deliberately does not launch a
Wayland GUI or drive UI export. Before publication, the host release gate MUST
therefore run the package with a fresh `CADENCE_DATA_DIR` under Wayland and
cover launch, event mutation, restart, notification, tray Show and Quit,
appearance, recurrence, recovery, and UI backup export (including its `jq`
assertions), along with the supported scaling and DST journeys.

## Manual publication and rollback

Install and exercise the exact draft package on a clean supported Ubuntu 26.04
Wayland machine. Publish it only after the host acceptance matrix passes,
including the actual GNOME/AppIndicator panel and the direct dbusmenu `Event`
route: Show restores a minimized window and Quit terminates Cadence. Record
failures against the individual audit item instead of treating package
lifecycle checks as GUI evidence.

If a release is withdrawn, mark the GitHub release as a draft again or delete
the release and tag. Removing the package does not remove timetable data; keep
the database and JSON backup unless the user explicitly chooses to archive or
delete them.
