# CADKernel Signing Guide

CADKernel release signing uses repository secrets. The workflows fail fast when
the required signing secrets are missing, so dry runs should use manual
dispatch only after secrets are configured.

## Required Secrets

Linux AppImage signing:

- `LINUX_GPG_PRIVATE_KEY`: ASCII-armored private key.
- `LINUX_GPG_PASSPHRASE`: passphrase for the private key.
- `LINUX_GPG_FINGERPRINT`: expected full fingerprint for the signing key.

macOS signing and notarization:

- `APPLE_CERTIFICATE_P12`: base64-encoded Developer ID Application certificate.
- `APPLE_CERTIFICATE_PASSWORD`: password for the `.p12` certificate.
- `APPLE_DEVELOPER_ID_APPLICATION`: codesigning identity name.
- `APPLE_ID`: Apple ID used for notarization.
- `APPLE_TEAM_ID`: Apple Developer Team ID.
- `APPLE_APP_SPECIFIC_PASSWORD`: app-specific password for notarization.

Windows Authenticode signing:

- `WINDOWS_CERTIFICATE_PFX`: base64-encoded code-signing certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`: password for the `.pfx` certificate.

## Key Handling

Store only encrypted or password-protected private keys in GitHub repository
secrets. Never commit certificates, private keys, passwords, or generated
keychains to the repository.

Rotate signing material when:

- a maintainer with release access leaves the project;
- a certificate or private key may have been exposed;
- the certificate authority requires renewal;
- the signing certificate is near expiration.

## Linux Verification

Download the AppImage, detached signature, and SHA-256 file from the GitHub
Release:

```bash
sha256sum -c cadkernel-1.0.0-x86_64.AppImage.sha256
gpg --verify cadkernel-1.0.0-x86_64.AppImage.asc \
  cadkernel-1.0.0-x86_64.AppImage
```

The signing fingerprint must match `LINUX_GPG_FINGERPRINT`.

## macOS Verification

Verify the DMG signature and notarization ticket:

```bash
codesign --verify --deep --strict --verbose=2 CADKernel.app
spctl --assess --type open --context context:primary-signature \
  cadkernel-1.0.0-macos-universal.dmg
xcrun stapler validate cadkernel-1.0.0-macos-universal.dmg
shasum -a 256 -c cadkernel-1.0.0-macos-universal.dmg.sha256
```

Gatekeeper should report an accepted Developer ID signature.

## Windows Verification

Verify the MSI signature and checksum in PowerShell:

```powershell
Get-AuthenticodeSignature .\cadkernel-1.0.0-x86_64.msi
Get-FileHash .\cadkernel-1.0.0-x86_64.msi -Algorithm SHA256
```

The Authenticode status should be `Valid`, and the hash should match the
published `.sha256` file.

## Release Review Checklist

- The tag matches the intended Cargo package version.
- `release.yml` raw binaries are attached to the GitHub Release.
- `sign.yml` produced all three signed installers.
- SHA-256 files exist for each installer.
- Linux AppImage GPG verification passes.
- macOS DMG notarization is stapled and validates.
- Windows MSI Authenticode verification passes.
- `release.json` exists and points only to assets from the same tag.
