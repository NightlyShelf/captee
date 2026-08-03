## 1. Git repository and GitHub publishing

- [x] 1.1 Initialize the local Git repository, add a safe `.gitignore`, and document branch and commit conventions.
- [x] 1.2 Authenticate with GitHub and verify the target repository under the user's account or organization.
- [x] 1.3 Add and verify the GitHub `origin` remote, commit the initial project state, and push the default branch without overwriting an existing remote.
- [ ] 1.4 Enable repository safeguards: protected default branch, required CI checks, dependency/security alerts, and least-privilege Actions permissions. (Alerts, Actions permissions, and CI are configured; branch protection is blocked by the current private-repository plan.)
- [x] 1.5 Document clone, authentication, contribution, release, and remote-recovery procedures in the project documentation.

## 2. Project foundation

- [x] 2.1 Create a Cargo workspace with `captee-core`, `captee-platform`, and `captee-ui` crates and document the crate boundaries.
- [x] 2.2 Add pinned Rust toolchain, formatting/lint configuration, dependency licenses, and feature gates so core tests build without GTK or a desktop session.
- [x] 2.3 Add the bundled Typst 0.14.2 binaries for supported targets with checksums, license notices, and a small wrapper exposing version and compile/format commands.
- [ ] 2.4 Add CI jobs for formatting, clippy, headless core tests, and an Ubuntu 22.04 x86_64 AppImage build using GTK 4.22.4.

## 3. Workspace management

- [x] 3.1 Define versioned project configuration, entry-document, recent-project, and settings models with validation and stable serialization.
- [x] 3.2 Implement project-root and relative-path validation, including traversal, symlink escape, unexpected file type, and invalid configuration checks.
- [x] 3.3 Implement atomic file replacement and revisioned autosave/recovery primitives with cleanup of failed temporary files.
- [x] 3.4 Implement create/open project flows, `img/` initialization, bounded deduplicated recents, and platform trash confirmation interfaces.
- [x] 3.5 Add unit tests for project creation/open rejection, path confinement, autosave recovery, recent-list behavior, and cancelled trash operations.

## 4. Typst authoring

- [ ] 4.1 Implement source-document state with dirty tracking, revision IDs, snapshots, undoable edits, and atomic save integration.
- [ ] 4.2 Implement bundled Typst diagnostic parsing into severity, message, and source-span values while preserving editable source on errors.
- [ ] 4.3 Implement debounced asynchronous compile/format scheduling and current-revision-only result application.
- [ ] 4.4 Implement formatter, completion, and literal find/replace services behind core traits with cancellation-safe outcomes.
- [ ] 4.5 Add tests for diagnostics, formatting failure preservation, stale-result rejection, completion cancellation, and confirmed replacement scope.

## 5. Preview and PDF export

- [ ] 5.1 Implement render state for current source revision, last successful preview, diagnostics, and render timestamps.
- [ ] 5.2 Implement asynchronous preview compilation using the bundled Typst adapter with stale-render rejection.
- [ ] 5.3 Implement atomic PDF export that refuses stale or missing successful renders and preserves an existing destination on failure.
- [ ] 5.4 Add fixture-based tests for successful preview, failed render retaining the last success, stale render rejection, and export refusal.

## 6. Screenshot capture and annotation

- [ ] 6.1 Define capture, annotation, and editor-insertion interfaces with explicit cancellation and failure outcomes.
- [ ] 6.2 Implement portal-first capture selection and bounded `grim`/`slurp` fallback subprocess adapters.
- [ ] 6.3 Implement pointer, rectangle, and text annotation operations over an in-memory image while preserving the original capture until confirmation.
- [ ] 6.4 Validate PNG output, generate collision-resistant project-relative asset names, and atomically save confirmed assets under `img/`.
- [ ] 6.5 Implement Typst image-expression insertion after successful asset storage, including the no-focused-editor outcome.
- [ ] 6.6 Add tests for portal/fallback selection, cancellation no-op behavior, malformed PNG rejection, atomic asset cleanup, and insertion formatting.

## 7. Desktop workspace UI

- [ ] 7.1 Implement a UI-agnostic application state store and command dispatcher for home, workspace, editor, preview, capture, and settings states.
- [ ] 7.2 Build the GTK 4.22.4 application shell with project home and accessible three-pane workspace using GtkSourceView for Typst editing.
- [ ] 7.3 Wire menus, keyboard shortcuts, save/format/find/capture/preview/export commands, and settings validation/persistence to the dispatcher.
- [ ] 7.4 Add non-blocking progress, cancellation, focus management, and accessible labels/status announcements for long-running operations and failures.
- [ ] 7.5 Add UI smoke tests or headless state-transition tests covering empty home, opened workspace, invalid settings, and failed operations.

## 8. Documentation and release review

- [ ] 8.1 Document project layout, recovery behavior, capture permissions, bundled Typst licensing, supported Linux session requirements, and troubleshooting in `README.md` and `docs/`.
- [ ] 8.2 Produce the x86_64 AppImage with bundled GTK runtime and Typst compiler, verify it on Ubuntu 22.04, and record reproducible build inputs.
- [ ] 8.3 Run the complete formatter, lint, unit, fixture, UI-state, and packaging checks; review each requirement scenario and record known limitations before release.
