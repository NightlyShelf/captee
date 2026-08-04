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
