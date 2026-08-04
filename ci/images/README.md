# Captee CI images

This directory defines two separate Linux x86_64 OCI images:

- `ghcr.io/nightlyshelf/captee/test` contains the Rust test toolchain and GTK
  development environment.
- `ghcr.io/nightlyshelf/captee/build` adds the AppImage packaging toolchain.

The inputs are declared in [`image-version.env`](image-version.env). The
`IMAGE_VERSION` is a human-readable cache/promotion label; CI consumers must
use the corresponding immutable digest, for example:

```text
ghcr.io/nightlyshelf/captee/test:2026.08.04.1@sha256:<validated-digest>
ghcr.io/nightlyshelf/captee/build:2026.08.04.1@sha256:<validated-digest>
```

The base image is pinned to the x86_64 Ubuntu 22.04 manifest digest recorded in
the version file. Package versions and downloaded packaging-tool checksums are
also inputs to the image version. Any change to those inputs, a Dockerfile, a
health check, or the publication workflow requires a new `IMAGE_VERSION`.

GHCR policy:

- Publish paths are `ghcr.io/nightlyshelf/captee/test` and
  `ghcr.io/nightlyshelf/captee/build`.
- The image publication job receives `packages: write`; consumer jobs receive
  only `packages: read`.
- Published digests are immutable and are never overwritten or deleted as a
  rollback mechanism.
- Keep at least 90 days of versioned images and retain every digest referenced
  by a protected branch or release metadata record.
- Rollback changes the consumer digest to a previously validated digest; it
  does not retag or mutate the failed image.

The local Docker daemon is not required to edit these inputs. Use the manual
image workflow for registry-backed builds when the local user cannot access
the Docker socket.
