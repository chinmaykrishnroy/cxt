# Package-manager distribution

GitHub Releases are the binary source of truth. Every release publishes `SHA256SUMS.txt`; installer scripts verify the selected asset before installation.

- Windows: submit the x64/ARM64 `.exe` assets to WinGet as `ChinmayKrishnRoy.cxt`.
- macOS: publish a Homebrew tap pointing to the architecture-specific `.tar.gz` release assets.
- Debian/Ubuntu: release workflow builds x64 and ARM64 `.deb` packages.
- Linux generic: architecture-specific `.tar.gz` assets are available for x64 and ARM64.

The Rust package is `cxt-ai`, while the executable/package-manager command remains `cxt`.

