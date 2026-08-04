## 1. Baseline and image inputs

- [x] 1.1 Measure current test and AppImage setup, pull, and total job times; record the current Rust, GTK/GtkSourceView, Ubuntu 22.04, and packaging-tool versions.
- [x] 1.2 Add versioned test/build image manifests with pinned base-image digests, package lists, Rust toolchain input, packaging-tool inputs, and cache-invalidation metadata.
- [x] 1.3 Define GHCR image naming, immutable digest metadata, retention, read/publish permissions, and rollback reference format.

## 2. Test environment image

- [x] 2.1 Create the x86_64 test image with pinned Rust, Cargo registry/git cache inputs, GTK 4/GtkSourceView development packages, pkg-config, and CI test utilities.
- [x] 2.2 Add test-image health checks for rustc/cargo/rustfmt/clippy, GTK/GtkSourceView pkg-config metadata, architecture, and noninteractive smoke commands.
- [x] 2.3 Add image publication and digest metadata output for the validated test image.

## 3. Build environment image

- [x] 3.1 Create the x86_64 build image with the Ubuntu 22.04-compatible GTK baseline, pinned Rust, Typst fetch prerequisites, AppImage/linuxdeploy/plugin/appimagetool/runtime prerequisites, squashfs, and patchelf.
- [x] 3.2 Add build-image health checks for toolchain, GTK/GtkSourceView, packaging commands, architecture, and a packaging preflight without emitting a release artifact.
- [x] 3.3 Add image publication and provenance metadata output for the validated build image.

## 4. Workflow integration

- [x] 4.1 Add a manual/release-controlled image publication workflow that builds both roles, uses registry layer caching, scans for secrets, validates digests, and publishes only after checks pass.
- [x] 4.2 Update `.github/workflows/rust-checks.yml` to run all test jobs in the pinned test image and remove repeated Rust/GTK setup where covered.
- [x] 4.3 Update `.github/workflows/appimage.yml` to run in the pinned build image while preserving manual-only triggering and artifact upload.
- [x] 4.4 Add one documented runner fallback/preflight path for image-pull failure and stale or unavailable digest failure.
- [x] 4.5 Preserve least-privilege GHCR permissions and ensure image consumers print and verify the resolved digest before running.

## 5. Performance and release validation

- [ ] 5.1 Run image smoke checks plus the complete fmt, clippy, workspace/UI test suite and manual AppImage packaging using the new images.
- [ ] 5.2 Compare setup, pull, and total job times with the baseline; capture cache hit/miss behavior and image-size trade-offs.
- [ ] 5.3 Review changed Docker/workflow code for CPU, memory, network/I/O, concurrency, layer lifetime, credential exposure, and cache-invalidation bottlenecks; record findings in the active change performance log and mirror architecture consequences in `docs/architecture.md`.
- [ ] 5.4 Document image promotion, digest updates, rollback, fallback, registry permissions, and local troubleshooting.
