## Context

The repository currently contains only the proposal and project documentation. The change spans project filesystem operations, Typst compilation, image capture, and a Linux desktop UI. The implementation must keep core behavior testable without GTK, a desktop session, Typst installed on PATH, or real filesystem side effects.

## Goals / Non-Goals

**Goals:**

- Establish a Rust workspace with a platform-independent core and narrow adapters for UI, capture, compiler, trash, and filesystem concerns.
- Make project writes safe by construction: validate relative paths, use temporary files plus atomic rename, and avoid mutation on cancellation or failed validation.
- Model source and render revisions so asynchronous compiler results cannot overwrite newer edits.
- Provide a vertical-slice desktop shell that can grow into the full GTK 4/GtkSourceView application without coupling domain logic to widgets.
- Keep a focused test suite for path validation, atomic persistence, revision handling, capture cancellation, and insertion formatting.

**Non-Goals:**

- Supporting Windows or macOS in the first release.
- Implementing a general-purpose Typst language server or full IDE feature set.
- Replacing the desktop portal, `grim`, `slurp`, or the Typst compiler; those remain adapters with test doubles.
- Synchronizing projects to a server or adding accounts, telemetry, or cloud storage.

## Decisions

### Workspace layout

Use a Cargo workspace with `crates/captee-core` for domain types and pure services, `crates/captee-platform` for filesystem/process/portal adapters, and `crates/captee-ui` for the GTK application shell. The core crate exposes traits rather than GTK or Linux types. This allows unit tests to run headlessly and keeps platform code replaceable.

### Project format

Store a small JSON configuration file at the project root, a configured entry `.typ` document, and screenshots under `img/`. Configuration contains only portable project settings; user-global recent projects and keybindings live in the platform configuration directory. Project-relative paths are canonicalized or lexically checked before every write.

### Persistence and recovery

Write configuration, Typst source, and generated PNG/PDF files to a same-directory temporary file, flush it, and atomically rename it into place. Autosave uses a separate revision-marked file so startup can offer recovery without silently replacing a newer user file. Failed writes clean up their temporary file and preserve the last complete destination.

### Async revision model

Every source edit increments a monotonically increasing revision. Compiler and renderer jobs receive a snapshot and revision; result application requires the revision to still be current. A debounced scheduler limits work during rapid typing, while cancellation is advisory and stale-result rejection is authoritative.

### Capture pipeline

Capture is represented as a staged value: backend output, annotations, confirmed PNG asset, and optional editor insertion. The UI can cancel before confirmation. Backend selection is portal-first with an explicit fallback setting. The core accepts PNG bytes and does not depend on a compositor for tests.

### UI boundary

GTK 4/GtkSourceView is an adapter over a state store and command dispatcher. Widgets subscribe to state snapshots and dispatch intent; they do not perform project writes, process spawning, or capture directly. Long-running work is scheduled off the UI thread and returns typed outcomes.

### Release targets and bundled compiler

The first desktop target is GTK 4.22.4, the current stable GTK 4 release, using the GTK 4.0 API namespace and a matching gtk4-rs binding. The x86_64 AppImage is built on Ubuntu 22.04 LTS to preserve a conservative glibc baseline while bundling the GTK runtime and application assets. The build must still run on both Wayland and X11 sessions where the capture adapters support them.

Typst is bundled rather than discovered on `PATH`. The initial compiler bundle is pinned to Typst 0.14.2, recorded with platform-specific checksums and license notices. Compiler upgrades are explicit dependency changes and must update golden diagnostic/preview fixtures.

### Per-task performance review gate

Every implementation task has a focused bottleneck review before its checkbox is marked complete. The review is limited to the task's changed code and considers CPU time, memory growth, filesystem and process I/O, concurrency/staleness, and resource lifetime. Findings, impact, mitigation, and follow-up are recorded in `performance-review.md` in this change. Architectural consequences are mirrored in `docs/architecture.md`; unresolved risks remain visible until they are mitigated or explicitly accepted before archive.

## Risks / Trade-offs

- [GTK and Typst platform dependencies may be unavailable in CI] → Keep the core crate dependency-light; gate platform/UI crates and run core tests by default.
- [Atomic rename semantics vary across filesystems] → Restrict writes to local project filesystems, flush before rename, and retain recovery files when durability cannot be confirmed.
- [External capture commands can hang or emit malformed data] → Use bounded subprocess timeouts, validate PNG signatures, and clean up partial output.
- [Large documents can make preview work expensive] → Debounce compilation and apply only current-revision results; expose a manual preview command.
- [Portable projects can be opened from hostile directories] → Reject path traversal, symlink escapes, unexpected file types, and unsafe configuration values before mutation.

## Migration Plan

This is a new repository with no existing project format. Introduce the project configuration and workspace crates together. Future format changes must include a version field, a read migration, and an atomic write-back only after successful validation. Rollback is removal of the new application; user project files remain ordinary Typst, JSON, and PNG/PDF files.

## Open Questions

None for the initial implementation scope. GTK 4.22.4, Ubuntu 22.04 LTS as the AppImage build baseline, and bundled Typst 0.14.2 are the selected defaults; later release work may revise them through a separate change.
