#!/bin/sh
# Installer for querypie-cli.
#
#   curl -fsSL https://raw.githubusercontent.com/sudosubin/querypie-cli/main/scripts/install.sh | sh
#
# Environment variables:
#   QUERYPIE_VERSION      Release tag to install (default: latest, e.g. v0.1.1)
#   QUERYPIE_INSTALL_DIR  Where to install the binary (default: $HOME/.local/bin)

set -eu

repo="sudosubin/querypie-cli"
dir="${QUERYPIE_INSTALL_DIR:-$HOME/.local/bin}"
version="${QUERYPIE_VERSION:-latest}"

case "$(uname -sm)" in
  "Darwin x86_64") target="x86_64-apple-darwin" ;;
  "Darwin arm64") target="aarch64-apple-darwin" ;;
  "Linux x86_64") target="x86_64-unknown-linux-gnu" ;;
  "Linux aarch64") target="aarch64-unknown-linux-gnu" ;;
  *) echo "querypie-cli: unsupported platform $(uname -sm)" >&2; exit 1 ;;
esac

archive="querypie-cli-$target.tar.xz"
if [ "$version" = latest ]; then
  base="https://github.com/$repo/releases/latest/download"
else
  base="https://github.com/$repo/releases/download/$version"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading querypie ($target)..." >&2
curl -fsSL "$base/$archive" -o "$tmp/$archive"
curl -fsSL "$base/$archive.sha256" -o "$tmp/$archive.sha256"

if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$archive")"
else
  actual="$(shasum -a 256 "$tmp/$archive")"
fi
expected="$(cut -d' ' -f1 <"$tmp/$archive.sha256")"
[ "${actual%% *}" = "$expected" ] || { echo "querypie-cli: checksum mismatch" >&2; exit 1; }

tar -xf "$tmp/$archive" -C "$tmp"
mkdir -p "$dir"
mv "$tmp/querypie-cli-$target/querypie" "$dir/querypie"
chmod +x "$dir/querypie"
echo "Installed querypie to $dir/querypie" >&2

case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "querypie-cli: $dir is not in your PATH" >&2 ;;
esac
