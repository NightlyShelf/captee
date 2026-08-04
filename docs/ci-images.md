# CI image operations

Captee publishes two separate x86_64 Ubuntu 22.04 images to GHCR:

- `ghcr.io/nightlyshelf/captee/test` contains the pinned Rust, GTK 4, and
  GtkSourceView development environment for formatting, linting, and tests.
- `ghcr.io/nightlyshelf/captee/build` contains the test environment plus Typst,
  linuxdeploy, its GTK plugin, appimagetool, the AppImage runtime, squashfs,
  FUSE, and ELF packaging tools.

## Promotion

Run the `CI images` workflow manually with `publish=true`. The workflow first
builds, health-checks, and secret-scans both images. The publication job then
pushes versioned tags with provenance and SBOM metadata and uploads
`published-images.json`. Treat the digest-qualified references in that file as
the only consumer inputs.

Update the repository variables after reviewing the artifact:

```sh
gh variable set CAPTEE_TEST_IMAGE --repo NightlyShelf/captee --body \
  'ghcr.io/nightlyshelf/captee/test:<version>@sha256:<digest>'
gh variable set CAPTEE_BUILD_IMAGE --repo NightlyShelf/captee --body \
  'ghcr.io/nightlyshelf/captee/build:<version>@sha256:<digest>'
```

The Rust checks read the test variable. The manual-only AppImage workflow reads
the build variable. Publication requires `packages: write`; consumers require
only `packages: read`.

## Rollback and fallback

Keep the previous digest-qualified references when promoting a new version. To
roll back, restore both repository variables to the previous compatible digests.
Consumer jobs print the selected digest and run a healthcheck before executing
commands. If a variable is absent or its image cannot be pulled or validated,
the workflows use the pinned runner setup when `CAPTEE_ALLOW_RUNNER_FALLBACK`
is enabled. This is slower because it repeats package installation, but it keeps
test and packaging availability during registry incidents.

## Local troubleshooting

Docker must be available locally. Load the manifest and run the healthcheck with
the same digest used by CI:

```sh
set -a
. ci/images/image-version.env
set +a
docker pull "${TEST_IMAGE}:${IMAGE_VERSION}"
docker run --rm --entrypoint /usr/local/bin/captee-test-healthcheck \
  "${TEST_IMAGE}:${IMAGE_VERSION}"
```

For a local source build, use the wrapper in runner mode or set the relevant
image variable and run `ci/images/prepare-environment.sh test` followed by
`ci/images/run-in-environment.sh cargo test --workspace`. A failed healthcheck
means the digest, architecture, or pinned inputs do not match; do not silently
replace an immutable reference with a mutable `latest` tag.
