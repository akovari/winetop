#!/usr/bin/env bash
# Write an updated AUR PKGBUILD for winetop-bin.
set -euo pipefail

VERSION="${1:?version required (no v prefix)}"
OUT="${2:-PKGBUILD}"

cat >"$OUT" <<EOF
# Maintainer: Adam Kovari <adam@kovari.eu>
pkgname=winetop-bin
pkgver=${VERSION}
pkgrel=1
pkgdesc="htop for Wine prefixes — monitor and kill Wine/Proton sessions"
arch=('x86_64' 'aarch64')
url="https://github.com/akovari/winetop"
license=('MIT')
provides=('winetop')
conflicts=('winetop')
options=('!strip')
source_x86_64=("\$pkgname-\$pkgver-x86_64.tar.xz::https://github.com/akovari/winetop/releases/download/v\$pkgver/winetop-v\$pkgver-x86_64-unknown-linux-gnu.tar.xz")
source_aarch64=("\$pkgname-\$pkgver-aarch64.tar.xz::https://github.com/akovari/winetop/releases/download/v\$pkgver/winetop-v\$pkgver-aarch64-unknown-linux-gnu.tar.xz")
sha256sums_x86_64=('SKIP')
sha256sums_aarch64=('SKIP')

package() {
  install -Dm755 winetop "\$pkgdir/usr/bin/winetop"
}
EOF
