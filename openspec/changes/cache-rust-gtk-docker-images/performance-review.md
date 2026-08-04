# Performance Review Log

## Task 1.1 — Baseline measurement

- **Finding:** Fresh GitHub-hosted runners repeat Rust toolchain and GTK package setup across parallel jobs. Normal successful Rust jobs spend 18–24 seconds installing GTK; the first clippy attempt in run 30932946612 spent 7m03s in that step and was cancelled after 7m18s. The successful AppImage run spent 28 seconds installing GTK/AppImage packages and 110 seconds total.
- **Impact:** Package-manager network and installation latency dominates the short test commands and can create multi-minute or stalled feedback. AppImage setup is also repeated before every manual package run.
- **Mitigation:** Move the pinned Rust, GTK/GtkSourceView, and packaging prerequisites into separate reusable test/build images, and retain the measured values as the comparison baseline.
- **Follow-up:** Record image pull time, registry cache hit/miss state, image size, and total job time after workflow migration. Confirm that image health checks fail before tests or packaging when the image is incomplete.

## Task 1.2 — Versioned image inputs

- **Finding:** The image definitions now share a small, explicit input manifest and separate package lists; package/tool versions and the x86_64 Ubuntu base digest are included in the cache key inputs.
- **Impact:** Exact package pins can make a build fail when an archive removes an old package revision, while unpinned inputs would silently change the environment.
- **Mitigation:** Keep the Ubuntu base digest and critical package versions in one reviewed manifest, fail image publication when the pins are unavailable, and require a new image version for any input change.
- **Follow-up:** Verify the pinned packages and base digest in the image publication workflow and document the required version bump.

## Task 1.3 — GHCR and rollback contract

- **Finding:** Test/build images have separate GHCR names, version labels, digest-only consumer references, retention guidance, and read/write permission boundaries.
- **Impact:** A mutable tag or broad registry token could make CI non-reproducible or expose credentials during image publication.
- **Mitigation:** Publish versioned tags only as labels, resolve and record digests, grant write access only to the publication job, and roll back by changing the consumer digest.
- **Follow-up:** Enforce the contract in the publication and consumer workflows and emit digest metadata as an artifact.

## Task 2.1 — Test image

- **Finding:** The test image installs the pinned Rust toolchain and GTK/GtkSourceView development packages and prefetches Cargo dependencies from workspace manifests without copying application source into the reusable image.
- **Impact:** The image has a larger one-time build and registry footprint, while source changes do not invalidate the dependency layer.
- **Mitigation:** Keep the test package set lean, copy only Cargo manifests for prefetching, and use separate role images so release tooling is not pulled by test jobs.
- **Follow-up:** Measure image size and pull time after the first publication and inspect Cargo cache hit behavior in test jobs.

## Task 2.2 — Test image health checks

- **Finding:** The test image validates x86_64, the pinned Rust toolchain/components, GTK/GtkSourceView pkg-config metadata, compiler availability, and locked Cargo metadata.
- **Impact:** A broken image can fail before consuming CI time on tests, but health checks add a small process startup cost to each consumer job.
- **Mitigation:** Keep checks noninteractive and bounded; run them once at job start and retain the Docker healthcheck for image-level validation.
- **Follow-up:** Verify the checks inside the published image and ensure failure output includes the observed versions.

## Task 3.1 — Build image

- **Finding:** The build image duplicates the compatible base/toolchain setup and adds verified AppImage tooling, FUSE, squashfs, and ELF packaging dependencies.
- **Impact:** The build image is materially larger than the test image and includes external packaging binaries that require periodic refresh.
- **Mitigation:** Keep it separately addressable, verify every downloaded binary by SHA-256 during image construction, and do not use it for normal test jobs.
- **Follow-up:** Measure image size/pull time and rotate packaging inputs only with a new image version.

## Task 3.2 — Build image health checks

- **Finding:** The build image checks toolchain/toolkit metadata, packaging executables, architecture, locked Cargo metadata, and noninteractive linuxdeploy/appimagetool version commands without creating an artifact.
- **Impact:** Preflight adds a small startup cost but prevents packaging with an incomplete or stale environment.
- **Mitigation:** Keep the preflight before AppImage packaging and report exact observed tool versions and image labels.
- **Follow-up:** Run the preflight in the publication workflow and manual AppImage job before the first package build.

## Tasks 2.3 and 3.3 — Publication and metadata

- **Finding:** The manual workflow validates local images before an optional publish job, uses separate Buildx cache scopes, emits registry digests from the publish action, and records source/base/tool inputs with provenance and SBOM generation enabled.
- **Impact:** Validation builds and publication builds can consume runner CPU twice on a publish request, while digest metadata makes downstream consumption auditable.
- **Mitigation:** Reuse GitHub Actions layer caches by role, keep publication opt-in, and upload the metadata artifact even when tags are later rolled back.
- **Follow-up:** Confirm registry digest output and artifact contents during the first manual publication.

## Task 4.1 — Image publication workflow

- **Finding:** Image publication is isolated in a manual-only workflow; secret scans and health checks run before the write-permission job.
- **Impact:** A manual publish can take longer because images are built once for validation and again for registry output, but invalid images cannot be promoted by the workflow path.
- **Mitigation:** Use Buildx GHA layer caches, separate test/build scopes, and least-privilege `packages:write`/`id-token:write` permissions only on the publish job.
- **Follow-up:** Measure validation/publish cache hit rates and registry push duration after the first run.

## Tasks 4.2 and 4.3 — Consumer workflow integration

- **Finding:** Rust checks and the manual-only AppImage workflow now run their commands through role-specific image wrappers and no longer install the normal toolchain/package set on the host when an image is configured.
- **Impact:** Every command incurs a short Docker process and workspace bind-mount overhead; the AppImage output path must be shared between host and container.
- **Mitigation:** Keep one image preflight per job, reuse the image's preloaded Cargo registry, mount only the checkout, and use a workspace-relative AppImage output directory.
- **Follow-up:** Compare Docker pull/process overhead against the baseline and confirm artifact upload from both image and fallback modes.

## Tasks 4.4 and 4.5 — Fallback and permissions

- **Finding:** Consumer jobs authenticate to GHCR with read-only `GITHUB_TOKEN` access, verify image readiness before commands, and use a pinned Ubuntu runner fallback when the image variable is unset or the pull/health check fails.
- **Impact:** Fallback preserves availability but can reintroduce the multi-minute package-install bottleneck; image variables must be updated deliberately after publication.
- **Mitigation:** Log the selected mode and exact image reference, keep fallback setup version-pinned, and limit registry write/OIDC permissions to the publication job.
- **Follow-up:** Validate both image and fallback branches in CI and document repository variable updates alongside digest metadata.
