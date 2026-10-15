# Package-manager distribution

GitHub Releases are the binary source of truth. Every release publishes `SHA256SUMS.txt`; installer scripts verify the selected asset before installation.

- Windows: submit the x64/ARM64 `.exe` assets to WinGet as `ChinmayKrishnRoy.cxt`.
- macOS: publish a Homebrew tap pointing to the architecture-specific `.tar.gz` release assets.
- Debian/Ubuntu: release workflow builds x64 and ARM64 `.deb` packages.
- Linux generic: architecture-specific `.tar.gz` assets are available for x64 and ARM64.

The Rust package is `cxt-ai`, while the executable/package-manager command remains `cxt`.

## Rebuilding an unpublished existing tag

If a workflow-only problem blocks an already-pushed tag, do not move that tag. Fix the workflow on `main`, then run **Release → Run workflow** on `main`, setting `release_tag` to the existing tag (for example `v0.5.0`). Validation, builds, installer scripts and notes are checked out from that immutable source tag. Its package version must match. Leave the input empty for build-only validation without publication. This is not a way to put changed application source under an old version.

Windows Rust commands must run in native PowerShell, not Git Bash: Bash can put its GNU `link.exe` ahead of Microsoft's linker. Packaging-only Bash steps do not invoke Rust. Hosted Windows CI and all native acceptance checks must still pass before publication.

## Opt-in development prereleases

Use an unused annotated `vMAJOR.MINOR.PATCH-dev.N` tag with a matching Cargo
version and `docs/releases/<tag>.md`. The workflow marks these releases as
prereleases and explicitly keeps them out of `latest`. Stable installer commands
and package-manager submissions remain on the stable release. Testing requires
an explicit tag, for example `CXT_VERSION=v0.5.2-dev.1` with the installer from
that tag. Installation replaces the chosen installation's binary; it is not a
side-by-side channel manager. Follow the release notes to restore stable.

Debian control metadata converts `0.5.2-dev.1` to `0.5.2~dev.1` so it sorts below
stable `0.5.2`; asset filenames retain the original Cargo version. A manually
installed prerelease `.deb` is still an intentional local opt-in. No prerelease
is automatically submitted to WinGet, a Homebrew tap or an APT repository.

Prereleases still require the local validation gates and hosted workflow. Their
notes must disclose incomplete behavior, known failures and which platform
checks are native versus cross-compiled. They are not stable-release acceptance.

