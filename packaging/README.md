# Package-manager distribution

GitHub Releases and the installers are the source of truth for `cxt`. Package managers should download the exact release assets and verify their SHA256 checksums.

## WinGet

WinGet submissions are free, but they must be submitted to Microsoft's `winget-pkgs` repository for validation and review.

Install the manifest creator:

```powershell
winget install Microsoft.WingetCreate
```

After publishing a release, generate a manifest from the release `.exe`:

```powershell
wingetcreate new https://github.com/chinmaykrishnroy/cxt/releases/download/vX.Y.Z/cxt-x86_64-pc-windows-msvc.exe
```

Use `ChinmayKrishnRoy.cxt` as the package identifier, validate the generated manifests, and submit them to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).

## Homebrew

Create a separate public repository named `homebrew-tap` and add `Formula/cxt.rb`. The formula should point to the macOS release archives, include architecture-specific URLs and SHA256 values from `SHA256SUMS.txt`, and install the binary into Homebrew's `bin` directory.

Users will then install it without administrator access:

```bash
brew install chinmaykrishnroy/tap/cxt
```

Homebrew handles PATH integration automatically.

## Debian and Ubuntu

The release workflow already creates `.deb` packages. Users can install one without adding a repository:

```bash
sudo dpkg -i cxt_X.Y.Z_amd64.deb
```

For `apt install cxt`, publish a signed APT repository containing the `.deb`, `Packages` indexes, and repository metadata. GitHub Pages can host the repository, but the repository must be signed and maintained. This is separate from submitting the package to the official Debian archive.

## Release checklist

1. Push a tag such as `v0.1.5`.
2. Wait for the GitHub Release workflow to publish all assets.
3. Copy the checksums into the WinGet manifest and Homebrew formula.
4. Submit the WinGet manifest.
5. Push the Homebrew formula to `homebrew-tap`.
6. Test `.deb` installation in a clean Debian/Ubuntu VM.
