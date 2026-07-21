#!/usr/bin/env bash
# Build a simple binary .deb from a GitHub Release tarball (no cargo needed).
# Usage:
#   ./dist/debian/build-deb-from-release.sh [version] [arch]
# Example:
#   ./dist/debian/build-deb-from-release.sh 0.1.0 amd64
set -euo pipefail

VERSION="${1:-0.1.0}"
ARCH="${2:-amd64}"

case "$ARCH" in
  amd64) TARGET=x86_64-unknown-linux-gnu ;;
  arm64) TARGET=aarch64-unknown-linux-gnu ;;
  *)
    echo "unsupported arch: $ARCH (use amd64 or arm64)" >&2
    exit 1
    ;;
esac

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

TARBALL_URL="https://github.com/akovari/winetop/releases/download/v${VERSION}/winetop-v${VERSION}-${TARGET}.tar.xz"
PKGDIR="$WORKDIR/winetop_${VERSION}-1_${ARCH}"

mkdir -p "$PKGDIR/DEBIAN" "$PKGDIR/usr/bin" "$PKGDIR/usr/share/man/man1" "$PKGDIR/usr/share/doc/winetop"

echo "Downloading $TARBALL_URL"
curl -fsSL "$TARBALL_URL" -o "$WORKDIR/winetop.tar.xz"
tar -xJf "$WORKDIR/winetop.tar.xz" -C "$WORKDIR"
install -m 0755 "$WORKDIR/winetop" "$PKGDIR/usr/bin/winetop"
install -m 0644 "$ROOT/man/winetop.1" "$PKGDIR/usr/share/man/man1/winetop.1"
install -m 0644 "$ROOT/README.md" "$PKGDIR/usr/share/doc/winetop/README.md"
install -m 0644 "$ROOT/CHANGELOG.md" "$PKGDIR/usr/share/doc/winetop/changelog"
gzip -9n "$PKGDIR/usr/share/doc/winetop/changelog"
gzip -9n "$PKGDIR/usr/share/man/man1/winetop.1"
install -m 0644 "$ROOT/LICENSE" "$PKGDIR/usr/share/doc/winetop/copyright"

cat >"$PKGDIR/DEBIAN/control" <<EOF
Package: winetop
Version: ${VERSION}-1
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: akovari <akovari@users.noreply.github.com>
Depends: libc6
Homepage: https://github.com/akovari/winetop
Description: htop for Wine prefixes
 Native CLI/TUI to monitor and stop Wine/Proton sessions.
EOF

OUT="$ROOT/dist/debian/winetop_${VERSION}-1_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$PKGDIR" "$OUT"
echo "Built $OUT"
echo "Install with: sudo apt install ./$(basename "$OUT")"
