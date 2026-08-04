## Purpose

Provide reproducible, reusable container environments that remove repeated
Rust and GTK setup work from Captee test and AppImage workflows while keeping
testing and release packaging isolated from one another.

## ADDED Requirements

### Requirement: Separate test and build environments

The CI system MUST publish two distinct Linux x86_64 container image roles:
one for workspace/UI testing and one for release/AppImage building. A workflow
MUST NOT use the release image as a substitute for the test image or vice
versa.

#### Scenario: Test workflow selects the test image

- **WHEN** a formatting, lint, workspace-test, or UI-state job starts
- **THEN** the job runs in the published test environment and has the pinned
  Rust toolchain and GTK/GtkSourceView development prerequisites available
  without repeating host package installation

#### Scenario: AppImage workflow selects the build image

- **WHEN** the manual AppImage package workflow starts
- **THEN** the job runs in the published build environment with Rust, GTK,
  GtkSourceView, AppImage packaging prerequisites, and release-tool fetch
  prerequisites available

### Requirement: Reuse immutable prepared environments

Each published image MUST be addressable by an immutable version and digest.
Consumer workflows MUST pin the image reference to a known-good digest or
recorded immutable version and MUST reuse the same image across all jobs with
the same role.

#### Scenario: Repeated test runs reuse the prepared toolchain

- **WHEN** two workflow runs use the same image digest
- **THEN** neither run reinstalls the Rust toolchain or GTK development
  packages as part of normal job setup

#### Scenario: Image changes invalidate reuse

- **WHEN** the Rust toolchain, GTK packages, packaging tools, base image, or
  image build definition changes
- **THEN** the image version changes and consumers do not silently continue
  using the previous image as the new environment

### Requirement: Validate image readiness before consumption

The image publication workflow MUST validate each image before publishing it,
including the pinned Rust toolchain, required GTK/GtkSourceView metadata, the
role-specific command-line tools, architecture, and a minimal noninteractive
smoke command.

#### Scenario: Invalid test image is rejected

- **WHEN** the test image is missing a required compiler, GTK development
  package, or smoke-test prerequisite
- **THEN** publication fails and no new consumer reference is promoted

#### Scenario: Valid build image is published

- **WHEN** the build image passes its architecture, toolchain, packaging-tool,
  and smoke checks
- **THEN** the registry receives the image with its immutable digest and the
  digest is exposed for workflow consumption

### Requirement: Preserve a controlled fallback

Consumer workflows MUST fail clearly or use the documented runner-based setup
when the selected image cannot be pulled, rather than using an untracked image
or silently omitting required Rust/GTK prerequisites.

#### Scenario: Registry outage during test setup

- **WHEN** a test job cannot pull the configured test image
- **THEN** the workflow uses the documented fallback setup or fails with an
  actionable image-pull error, and the job still verifies its toolchain before
  running tests

#### Scenario: Stale build image reference

- **WHEN** the configured build image digest is unavailable or fails its
  readiness check
- **THEN** the AppImage workflow stops before packaging and reports the exact
  image reference and remediation path

### Requirement: Restrict image publication and record provenance

Image publication MUST use least-privilege registry permissions, MUST avoid
embedding credentials or project secrets in layers, and MUST record the source
commit, base image, toolchain/package inputs, architecture, and resulting image
digests.

#### Scenario: Image provenance is available for a release

- **WHEN** a build image is promoted for AppImage packaging
- **THEN** the workflow exposes the digest and provenance metadata needed to
  reproduce or audit the environment

#### Scenario: Secret scan detects a credential in a layer

- **WHEN** image validation finds a credential or forbidden secret path in an
  image layer
- **THEN** publication fails and the image is not referenced by CI workflows
