#!/usr/bin/env bash
# Generate a Homebrew formula with version + sha256 from release assets.
set -euo pipefail

VERSION="${1:?version required (no v prefix)}"
SHA_AMD64="${2:?sha256 amd64}"
SHA_ARM64="${3:?sha256 arm64}"
OUT="${4:-Formula/winetop.rb}"

mkdir -p "$(dirname "$OUT")"
cat >"$OUT" <<EOF
class Winetop < Formula
  desc "htop for Wine prefixes"
  homepage "https://github.com/akovari/winetop"
  version "${VERSION}"
  license "MIT"

  on_linux do
    on_intel do
      url "https://github.com/akovari/winetop/releases/download/v#{version}/winetop-v#{version}-x86_64-unknown-linux-gnu.tar.xz"
      sha256 "${SHA_AMD64}"
    end
    on_arm do
      url "https://github.com/akovari/winetop/releases/download/v#{version}/winetop-v#{version}-aarch64-unknown-linux-gnu.tar.xz"
      sha256 "${SHA_ARM64}"
    end
  end

  def install
    bin.install "winetop"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/winetop --version")
  end
end
EOF
