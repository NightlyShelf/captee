## Why

The current GitHub Actions jobs repeatedly install Rust dependencies and GTK
development packages on fresh runners. This makes test and AppImage feedback
slower and duplicates network, package-manager, and compilation work. Reusable
versioned Docker images can move that cost to image publication and let normal
checks start from a prepared environment.

## What Changes

- Add a reusable Linux x86_64 test image containing the pinned Rust toolchain,
  Cargo configuration/cache inputs, GTK 4 and GtkSourceView development
  packages, and the tools required by workspace and UI-state tests.
- Add a separate reusable Linux x86_64 build image containing the pinned Rust
  toolchain, GTK runtime/development and AppImage packaging prerequisites,
  bundled compiler fetch prerequisites, and release packaging tools.
- Build and publish the two images through an explicit versioned workflow with
  immutable tags/digests and architecture validation.
- Update Rust test and manual AppImage workflows to consume the appropriate
  image instead of reinstalling the same toolkit on every job.
- Define cache invalidation inputs, image health checks, retention, and a
  documented fallback to the existing runner setup when an image is unavailable.

## Capabilities

### New Capabilities

- `ci-build-environments`: Provides separate, versioned Docker environments for
  fast headless/UI testing and manual Linux AppImage packaging.

### Modified Capabilities

- None.

## Impact

- Affected workflows: `.github/workflows/rust-checks.yml` and
  `.github/workflows/appimage.yml`, plus a new image-build/publish workflow.
- New Dockerfiles, image metadata, health checks, and registry credentials or
  permissions are required.
- CI becomes dependent on the selected container registry and immutable image
  digests; runner fallback behavior must preserve reproducibility.
- Local source code and application runtime behavior are unchanged.
