# Launchpad PPA

**PPA:** [`ppa:kovariadam/winetop`](https://launchpad.net/~kovariadam/+archive/ubuntu/winetop)  
**Packages page:** https://launchpad.net/~kovariadam/+archive/ubuntu/winetop/+packages  

**Signing key fingerprint:** `A527 AE5A 9746 F3D9 54CA  8F4C 9C7E 01C1 5210 C325`  
**Key ID:** `9C7E01C15210C325`

## GitHub Actions secrets (on `akovari/winetop`)

| Secret | Value |
|--------|--------|
| `LAUNCHPAD_PPA` | `ppa:kovariadam/winetop` |
| `LAUNCHPAD_USER` | `kovariadam` |
| `LAUNCHPAD_GPG_FINGERPRINT` | `A527AE5A9746F3D954CA8F4C9C7E01C15210C325` |
| `LAUNCHPAD_GPG_KEY_ID` | `9C7E01C15210C325` |
| `LAUNCHPAD_GPG_PRIVATE_KEY` | ASCII-armored secret key (see below) |
| `LAUNCHPAD_GPG_PASSPHRASE` | Key passphrase (empty string if none) |

### Export and set the private key

```bash
FPR=A527AE5A9746F3D954CA8F4C9C7E01C15210C325

# 1) Metadata secrets (no private material)
gh secret set LAUNCHPAD_PPA --repo akovari/winetop --body "ppa:kovariadam/winetop"
gh secret set LAUNCHPAD_USER --repo akovari/winetop --body "kovariadam"
gh secret set LAUNCHPAD_GPG_FINGERPRINT --repo akovari/winetop --body "$FPR"
gh secret set LAUNCHPAD_GPG_KEY_ID --repo akovari/winetop --body "9C7E01C15210C325"

# 2) Private key (prompts for passphrase if the key is protected)
gpg --armor --export-secret-keys "$FPR" | \
  gh secret set LAUNCHPAD_GPG_PRIVATE_KEY --repo akovari/winetop

# 3) Passphrase (skip or use empty if the key has no passphrase)
gh secret set LAUNCHPAD_GPG_PASSPHRASE --repo akovari/winetop
# then paste passphrase and press Ctrl-D, or:
# printf '%s' 'your-passphrase' | gh secret set LAUNCHPAD_GPG_PASSPHRASE --repo akovari/winetop
```

Confirm the **public** key is on Launchpad:  
https://launchpad.net/~kovariadam/+editpgpkeys  

And published to the Ubuntu keyserver if you have not already:

```bash
gpg --send-keys --keyserver keyserver.ubuntu.com A527AE5A9746F3D954CA8F4C9C7E01C15210C325
```

## Do you need Launchpad API credentials?

**Usually no** for uploading packages. Uploads use `dput` + GPG; FTP login is anonymous and authenticity is the signature.

**Only if** CI must manage Launchpad via API (create PPAs, query builds, etc.):

1. Install a helper, e.g. `ppa-dev-tools` or [lpcli](https://github.com/canonical/lpcli)
2. Run `ppa credentials create` or `lpcli login` (browser OAuth)
3. Store the resulting access token / secret as optional secrets later

See: https://documentation.ubuntu.com/launchpad/user/how-to/launchpad-api/launchpad-web-signing/

## Install from this PPA (once packages are published)

```bash
sudo add-apt-repository ppa:kovariadam/winetop
sudo apt update
sudo apt install winetop
```
