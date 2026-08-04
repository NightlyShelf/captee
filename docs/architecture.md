# Architecture

Captee is organized as a Cargo workspace with three crates:

| Crate | Responsibility | Constraints |
| --- | --- | --- |
| `captee-core` | Project models, editor state, revision handling, validation, and pure services | Must remain headless and free of GTK, Linux, process, and real-filesystem dependencies |
| `captee-platform` | Filesystem, atomic persistence, bundled Typst, portal/capture, trash, and subprocess adapters | Depends on core; platform effects are exposed through narrow interfaces and test doubles |
| `captee-ui` | GTK 4 desktop shell, commands, state presentation, and accessibility integration | Depends on core and platform; widgets do not own project or process side effects |

The dependency direction is intentionally one-way: UI → platform → core. Core
tests must be runnable without a desktop session, GTK development packages, or
the bundled Typst executable. Long-running compiler, renderer, and capture work
will be scheduled outside the GTK main loop and applied through revision-checked
results.

## Performance considerations

The initial source editor stores complete text snapshots for undo and redo. This
keeps the implementation simple and reliable for ordinary notes, but memory use
can grow approximately with document size multiplied by the number of edits.
Large documents or long editing sessions may therefore become a bottleneck.

Before optimizing for large documents, history should move to bounded edit
records containing ranges and inserted/removed text, coalesce continuous typing,
and enforce a byte budget. Periodic full snapshots can remain as recovery and
fast-replay checkpoints. Any replacement must preserve the same undo, redo, and
revision semantics covered by the core tests.

## Implementation review gate

Performance reviews are scoped to the code changed by each implementation task,
not an undifferentiated scan of the entire repository. Before a task is marked
complete, record its CPU, memory, filesystem/process I/O, concurrency, and
resource-lifetime risks plus mitigations in the active OpenSpec change's
`performance-review.md`. Mirror any architectural consequence here, and keep
unresolved risks visible until they are mitigated or explicitly accepted before
the change is archived.

Compiler diagnostic parsing currently materializes one owned message and path
per accepted line. This is linear in compiler output and appropriate for normal
error counts, but very large or repetitive output can increase allocation and
rendering cost; future integrations should cap displayed diagnostics and parse
streams incrementally when that becomes observable.

The revision scheduler keeps only the newest pending source snapshot, waits for
the debounce interval, and accepts worker results only when their revision still
matches the current document. This bounds queued work and prevents stale UI
updates, while each submission still temporarily owns a source-string snapshot.

Render state follows the same revision boundary: it retains the last successful
PDF preview after a failed render, exposes diagnostics for the current attempt,
and accepts timestamps and results only for the active source revision. The core
state does not perform compiler, filesystem, or thread work; those remain
platform-side responsibilities for the preview scheduler.

The platform preview adapter stages each in-memory source snapshot in a unique
project-local temporary file so relative Typst assets resolve as they do for the
entry document. It invokes the bundled Typst runner on a worker thread, reads
the generated PDF into a revision-tagged outcome, and removes both temporary
files on every completion path. Applying the outcome through core render state
is the authoritative stale-result check.

PDF export reads only the preview whose revision matches the active source and
rejects missing or stale previews before touching the destination. It validates
the destination boundary and delegates the final write to the same flushed,
temporary-file-plus-rename primitive used by project persistence, so a failed
write leaves an existing PDF intact.

Headless fixture tests exercise the preview worker and export boundary with a
compiler test double, covering successful output, failed-render retention,
stale-result rejection, and refusal to export after a source revision changes.

Capture flow contracts are also defined in core without image, compositor, or
editor dependencies. Capture, annotation, and insertion adapters return typed
completion, cancellation, failure, and no-focused-editor outcomes; portal,
fallback subprocess, image rendering, and editor-widget behavior remain in the
platform and UI crates.

The platform capture selector tries the portal adapter first, treats portal
cancellation as a no-op, and uses the configured `grim`/`slurp` fallback only
after a portal failure. Fallback subprocesses are polled with a timeout and
terminated on expiry; their output is still unvalidated raw image data until
the PNG-validation task.

The platform annotation adapter decodes each captured PNG into a bounded RGBA
surface, applies clipped pointer, rectangle, or fixed-glyph text marks, and
encodes a separate staged PNG. The original capture is borrowed immutably, so
cancellation before confirmation cannot change capture bytes. A 16-million
pixel limit bounds the adapter's decoded surface; the asset-storage boundary
must still validate the final PNG and enforce its own byte budget.

Authoring services are trait boundaries so formatting and completion can run in
platform workers rather than the UI thread. Literal find/replace creates a new
result string on confirmation and intentionally performs no allocation when a
replacement is cancelled; large replacements may still create temporary peak
memory proportional to the document and replacement size.

Completion cancellation is checked before and after a provider call, so a late
result cannot be applied after cancellation even when the provider itself cannot
be interrupted. Regression tests cover this stale-result boundary and formatter
failure preservation.

CI runs formatting, clippy, and headless core tests as separate jobs with a
shared Rust cache and read-only repository permissions. Parallel jobs shorten
feedback time but may duplicate dependency compilation; the AppImage job will
reuse the same cache and add GTK/packaging work only after the desktop crate is
ready.

The CI quality gates intentionally fail on formatting drift and clippy warnings,
so the pinned toolchain and committed lockfile are part of the reproducible build
boundary rather than optional local conventions.
