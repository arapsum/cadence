# Cadence release procedure

Cadence publishes Ubuntu x86_64 `.deb` artifacts from version tags. Releases
are intentionally created as drafts so a maintainer can install the artifact
and complete the manual smoke test before publishing it.

## Prepare a release

1. Update the workspace version in the root `Cargo.toml`.
2. Add a matching `## [version] - YYYY-MM-DD` section to `CHANGELOG.md`.
3. Update the AppStream release entry in
   `packaging/linux/io.github.arapsum.Cadence.metainfo.xml`.
4. Run the local checks:

   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace --locked --all-targets --all-features -- -D warnings \
     -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms
   cargo test --workspace --locked --all-targets --all-features
   ```

5. Commit the release preparation and create an annotated tag:

   ```sh
   git tag -a v<version> -m "Cadence <version>"
   git push origin main v<version>
   ```

## Automated release

The `Release` workflow verifies that the tag, Cargo version, changelog, and
metadata agree. It builds the locked release binary, creates a deterministic
`.deb`, validates desktop/AppStream metadata and package contents, and runs a
clean Ubuntu installation/upgrade/uninstall check. It then publishes SHA-256
checksums and build-provenance attestations and creates a draft GitHub release.

The equivalent local commands are:

```sh
scripts/package-linux.sh
scripts/validate-linux-package.sh target/dist/cadence_<version>_amd64.deb
scripts/verify-linux-install.sh target/dist/cadence_<version>_amd64.deb
```

The container check requires Docker and network access to Ubuntu's package
repositories. If Docker is unavailable, perform the same install, upgrade,
launch, backup, and uninstall checks manually on a clean Ubuntu 26.04 machine.

## Manual publication and rollback

Install the draft package on a clean supported machine, launch it under Wayland,
create a test event, restart the app, export a backup, and verify the package's
desktop entry, icon, About dialog, and `cadence --version` output. Publish the
draft only after those checks pass.

If a release is withdrawn, mark the GitHub release as a draft again or delete
the release and tag. Removing the package does not remove timetable data; keep
the database and JSON backup unless the user explicitly chooses to archive or
delete them.
