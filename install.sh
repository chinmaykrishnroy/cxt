#!/usr/bin/env sh
set -eu
repo="chinmaykrishnroy/cxt"
version="${CXT_VERSION:-latest}"
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in Linux) platform="unknown-linux-gnu" ;; Darwin) platform="apple-darwin" ;; *) echo "Unsupported OS: $os" >&2; exit 1 ;; esac
case "$arch" in x86_64|amd64) target="x86_64-$platform" ;; arm64|aarch64) target="aarch64-$platform" ;; *) echo "Unsupported architecture: $arch" >&2; exit 1 ;; esac
archive="cxt-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM
curl --fail --location --proto '=https' --tlsv1.2 "https://github.com/$repo/releases/$version/download/$archive" -o "$tmp/$archive"
tar -xzf "$tmp/$archive" -C "$tmp"
install_dir="${CXT_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$install_dir"
install "$tmp/cxt-$target" "$install_dir/cxt"
echo "Installed cxt to $install_dir/cxt"
