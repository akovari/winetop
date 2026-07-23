# Releasing winetop

Push a version tag to trigger [.github/workflows/release.yml](../.github/workflows/release.yml):

```bash
git tag v0.1.0
git push origin v0.1.0
```

Or: GitHub → Actions → Release → Run workflow → enter `v0.1.0`.

## What CI publishes

| Channel | Job | Secrets used |
|---------|-----|----------------|
| GitHub Release (tarballs, `.deb`, installer, checksums) | `release` | `GITHUB_TOKEN` (automatic) |
| crates.io (`winetop-core`, `winetop`) | `crates` | `CARGO_REGISTRY_TOKEN` |
| Homebrew tap `akovari/homebrew-tap` | `homebrew` | `HOMEBREW_TAP_SSH_KEY`, `HOMEBREW_TAP_REPO` |
| AUR `winetop-bin` | `aur` | `AUR_SSH_PRIVATE_KEY` |
| Fedora Copr `$COPR_USERNAME/winetop` | `copr` | `COPR_LOGIN`, `COPR_TOKEN`, `COPR_USERNAME` |
| Launchpad `ppa:kovariadam/winetop` | `launchpad` | `LAUNCHPAD_*` (continue-on-error) |

## One-time manual setup (verify before first tag)

1. **Copr project** exists: https://copr.fedorainfracloud.org/coprs/kovariadam/winetop/ — enable network for cargo builds.
2. **AUR**: `AUR_SSH_PRIVATE_KEY` must be an OpenSSH private key (begin with `-----BEGIN OPENSSH PRIVATE KEY-----`), with real newlines (not `\n` literals), and the matching public key registered on your AUR account. First publish creates `winetop-bin`.
3. **Launchpad**: GPG public key on https://launchpad.net/~kovariadam/+editpgpkeys matching fingerprint `A527 AE5A … 5210 C325`.
4. **crates.io**: token owner can publish new crate names `winetop` and `winetop-core` (first publish claims the name).

## Install after release

End-user install matrix: [../README.md](../README.md#install).

```bash
# GitHub installer
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/akovari/winetop/releases/latest/download/winetop-installer.sh | sh

brew install akovari/tap/winetop
# Arch: yay -S winetop-bin
sudo dnf copr enable kovariadam/winetop && sudo dnf install winetop
sudo add-apt-repository ppa:kovariadam/winetop && sudo apt install winetop
cargo install winetop --locked
```
