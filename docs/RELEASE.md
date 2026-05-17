# CADKernel Release Workflow

This document describes the tag-to-release path for CADKernel desktop builds.
The release infrastructure is split into three workflows:

- `.github/workflows/release.yml` builds reproducible raw binaries.
- `.github/workflows/sign.yml` builds and signs Linux, macOS, and Windows installers.
- `.github/workflows/auto-update.yml` publishes `release.json` for update clients.

## Release Tag

1. Ensure `Cargo.toml` and user-facing documentation describe the intended
   version.
2. Run the full verification gate locally:

   ```bash
   cargo build --workspace && \
     cargo clippy --workspace --all-targets --all-features -- -D warnings && \
     cargo test --workspace
   ```

3. Create an annotated version tag:

   ```bash
   git tag -a v1.0.0 -m "CADKernel v1.0.0"
   git push origin v1.0.0
   ```

## Build Artifacts

`release.yml` runs on tag pushes and manual dispatch. It installs Rust
`1.85.0`, sets `SOURCE_DATE_EPOCH` from the tagged commit timestamp, disables
incremental compilation, strips release debuginfo, and runs `cargo build
--locked --release` for:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Each binary is renamed with the version and target triple, hashed with
SHA-256, uploaded as a workflow artifact, then attached to the GitHub Release.

## Signed Installers

`sign.yml` also runs on tag pushes and manual dispatch. It builds:

- Linux: `cadkernel-{version}-x86_64.AppImage`
- macOS: `cadkernel-{version}-macos-universal.dmg`
- Windows: `cadkernel-{version}-x86_64.msi`

The Linux job creates an AppDir with `linuxdeploy`, packages it with
`appimage-builder`, signs the AppImage with GPG, and uploads the AppImage,
detached signature, and SHA-256 file.

The macOS job builds x86_64 and arm64 binaries, combines them with `lipo`,
codesigns the `.app` bundle, creates a `.dmg` with `create-dmg`, signs and
notarizes the DMG, staples the notarization ticket, and uploads the DMG plus
SHA-256 file.

The Windows job signs the executable, builds an MSI with `cargo-wix` using
`release/wix-template.wxs`, signs the MSI with SignTool, and uploads the MSI
plus SHA-256 file.

## Auto-Update Manifest

`auto-update.yml` runs on tag pushes and manual dispatch. On tag pushes it may
start before all signed assets are available, so it waits for the AppImage, DMG,
and MSI assets to appear on the GitHub Release.

After the signed assets are present, it downloads them, computes SHA-256
digests, fills `release/release.json.template`, and uploads:

- `release.json`
- `release.json.sha256`

The manifest contains the release version, notes URL, platform-specific
download URLs, installer type, file name, and SHA-256 digest.

## Manual Re-Run

For a failed release job, rerun the failed GitHub Actions job when possible. To
recreate assets for an existing tag, use workflow dispatch with the exact tag,
for example `v1.0.0`.

Manual dispatch never changes the source tree. It checks out the tag, rebuilds
the requested artifacts, and updates the matching GitHub Release assets.
