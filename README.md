<p align="center">
  <img src="app/static/icon.svg" width="128" height="128" alt="ghui icon">
</p>

<h1 align="center">ghui</h1>

[![CI](https://github.com/damyanp/ghui/actions/workflows/rust.yml/badge.svg)](https://github.com/damyanp/ghui/actions/workflows/rust.yml)
[![Build Windows Installer](https://github.com/damyanp/ghui/actions/workflows/build-installer.yml/badge.svg)](https://github.com/damyanp/ghui/actions/workflows/build-installer.yml)

## Windows Code Signing

The installer is code-signed in CI to avoid the Windows Defender SmartScreen
warning ("Windows protected your PC"). To enable signing, add the following
[repository secrets](https://docs.github.com/en/actions/security-for-github-actions/security-guides/using-secrets-in-github-actions):

| Secret | Description |
|--------|-------------|
| `WINDOWS_CERTIFICATE_BASE64` | Base64-encoded `.pfx` code signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx` file |

### Generating the secrets

1. Obtain a Windows Authenticode code signing certificate (OV or EV) from a
   trusted Certificate Authority.
2. Export or convert it to a `.pfx` (PKCS#12) file containing the private key.
3. Base64-encode the `.pfx` file and store the result as
   `WINDOWS_CERTIFICATE_BASE64`:
   ```bash
   base64 -i certificate.pfx | tr -d '\n'          # macOS / Linux
   [Convert]::ToBase64String([IO.File]::ReadAllBytes("certificate.pfx"))  # PowerShell
   ```
4. Store the `.pfx` password as `WINDOWS_CERTIFICATE_PASSWORD`.

When these secrets are configured, the **Build Windows Installer** workflow will
automatically sign the installer. Without them, the workflow still succeeds but
produces an unsigned installer.

> **Note:** An EV (Extended Validation) certificate provides immediate
> SmartScreen trust. An OV (Organization Validation) certificate builds trust
> gradually as the signed application gains reputation.
