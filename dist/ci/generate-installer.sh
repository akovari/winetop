#!/usr/bin/env bash
# Generate a portable install script for GitHub Releases.
set -euo pipefail

VERSION="${1:?version required (no v prefix)}"
OUT="${2:-winetop-installer.sh}"

cat >"$OUT" <<EOF
#!/bin/sh
# winetop installer — downloads the latest (or pinned) release binary
set -eu
VERSION="${VERSION}"
REPO="akovari/winetop"
BIN_DIR="\${BIN_DIR:-\${CARGO_HOME:-\$HOME/.cargo}/bin}"
mkdir -p "\$BIN_DIR"

arch=\$(uname -m)
case "\$arch" in
  x86_64|amd64) target=x86_64-unknown-linux-gnu ;;
  aarch64|arm64) target=aarch64-unknown-linux-gnu ;;
  *) echo "unsupported arch: \$arch" >&2; exit 1 ;;
esac

os=\$(uname -s)
case "\$os" in
  Linux) ;;
  *) echo "this installer currently supports Linux only (got \$os)" >&2; exit 1 ;;
esac

url="https://github.com/\$REPO/releases/download/v\${VERSION}/winetop-v\${VERSION}-\${target}.tar.xz"
tmp=\$(mktemp -d)
trap 'rm -rf "\$tmp"' EXIT
echo "Downloading \$url"
curl -fsSL "\$url" | tar -xJ -C "\$tmp"
install -m 0755 "\$tmp/winetop" "\$BIN_DIR/winetop"
echo "Installed \$BIN_DIR/winetop"
"\$BIN_DIR/winetop" --version || true
EOF
chmod +x "$OUT"
