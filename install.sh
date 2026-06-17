#!/usr/bin/env sh
# Aftercode CLI installer for macOS / Linux.
#   curl -fsSL https://raw.githubusercontent.com/ron3899/aftercode/main/install.sh | sh
# Downloads the latest prebuilt binary — no Rust or build tools required.
# (macOS users can also: brew install ron3899/aftercode/aftercode)
set -eu

repo="ron3899/aftercode"
os="$(uname -s)"
arch="$(uname -m)"

case "$os-$arch" in
  Darwin-arm64)  target="aarch64-apple-darwin" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  Linux-x86_64)  target="x86_64-unknown-linux-gnu" ;;
  *)
    echo "No prebuilt binary for $os-$arch." >&2
    echo "Install with Rust instead: cargo install --git https://github.com/$repo aftercode-cli" >&2
    exit 1 ;;
esac

asset="aftercode-${target}.tar.gz"
url="https://github.com/${repo}/releases/latest/download/${asset}"
dest="${AFTERCODE_BIN:-$HOME/.local/bin}"

echo "Downloading $url"
mkdir -p "$dest"
tmp="$(mktemp -d)"
curl -fsSL "$url" -o "$tmp/$asset"
tar -C "$dest" -xzf "$tmp/$asset"
chmod +x "$dest/aftercode"
rm -rf "$tmp"

echo ""
echo "Installed aftercode to $dest/aftercode"
case ":$PATH:" in
  *":$dest:"*) ;;
  *) echo "Add $dest to your PATH, e.g.:  export PATH=\"$dest:\$PATH\"" ;;
esac
echo "Next: start the backend with Docker, then run:  aftercode login"
