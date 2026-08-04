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

The application state store also lives in core. It exposes immutable snapshots
and a typed dispatcher for navigation, project context, dirty state, settings,
operation activity, and cancellation. GTK adapts those snapshots to widgets;
the store performs no widget, filesystem, process, or thread work.

The UI crate adds a presentation adapter over those snapshots. It owns logical
pane selection, keyboard-action declarations, focus targets, progress, and
accessible status announcements, while command execution remains routed through
the core dispatcher. The GTK adapter uses the GTK 4 API surface and
GtkSourceView 5 for the source editor. It is validated locally against GTK
4.22.4 while keeping the used API subset compatible with the Ubuntu 22.04
runner's GTK 4.6.9 packages;
headless state tests remain independent of a desktop session. Native startup
renders the headless `Home` state as an explicit Welcome surface; the folder
chooser then loads a validated project and switches to the three-pane workspace.
The current open/create callback reads the project configuration and source
document synchronously after the asynchronous chooser returns, so unusually
large entry documents can briefly block the GTK loop; this should move to a
  bounded worker when large-project support is expanded. The chooser uses the
  GTK 4.6-compatible native dialog API so the release builder does not require
  a newer host development package.

The UI operation coordinator is the lifetime boundary between GTK callbacks and
worker-owned platform effects. It assigns a new generation whenever a project
is activated, tags work with the active source revision and a unique operation
identity, and returns terminal outcomes through a non-blocking result channel.
Project changes, revision changes, explicit cancellation, and coordinator drop
signal cooperative cancellation; late results are classified as stale before
they can reach widgets or application state. The coordinator performs no I/O
itself. Platform adapters remain responsible for observing cancellation around
blocking calls and terminating subprocesses they own.

Project creation is presented as a modal name-and-parent-location form, while
opening uses the GTK 4.6-compatible native folder chooser. Both successful
paths route through the same workspace transition, and closing routes through
the corresponding home transition so the editor buffer and project label cannot
drift from the headless application state. Project-only menu buttons are built
inside the workspace surface; the home screen keeps only its create/open
actions. Dialog callbacks perform only the user-requested project filesystem
operation and retain the existing follow-up for moving large project loads off
the GTK main loop.

## Performance considerations

Operation coordination keeps one active handle and uses constant-time identity
checks. A worker can enqueue only one terminal result through its single-use
task handle, which bounds channel growth by the number of retired workers rather
than their internal progress. GTK integration must poll results from the main
context without a busy loop and avoid synchronously joining long-running work.
Headless integration tests use deterministic worker doubles against this public
boundary, covering terminal outcomes and stale project/revision rejection
without requiring GTK, threads, or platform processes.

The GtkSourceView bridge owns one core `SourceDocument` for the active entry
file. Buffer changes update that document, propagate its revision to the
operation coordinator, and mirror dirty state into the application store.
Programmatic open, undo, redo, and close updates suppress recursive buffer
signals. Undo and redo are routed through the core document history so widget
state cannot diverge from revision and dirty semantics.

Project persistence is exposed by a platform adapter that resolves the active
entry file inside `ProjectPaths` and implements core `DocumentPersistence` with
atomic replacement. Manual save runs off the GTK thread and returns the saved
core document through the revision-tagged operation channel; only that matching
document can clear dirty state. A 750 ms sequence-based debounce writes a
revisioned project-local autosave on a worker, and successful manual save removes
it. Project open compares a complete autosave with disk source and requires an
explicit modal decision before restoring it as unsaved editor content. Recent
projects use a bounded, deduplicated JSON store under the GLib user-data path.
Background persistence results carry project identity and cannot update a
different project after navigation or window teardown.

Authoring actions use the same revision boundary. Formatting stages the active
source in a collision-resistant project-local temporary file and invokes the
discovered packaged, development, or PATH Typst binary on a worker. Successful
formatted text is one undoable core edit; failures retain source and render up
to 20 structured diagnostics. Literal replacement is explicitly confirmed and
applied as one core edit. Completion uses a testable provider, converts GTK
character offsets to UTF-8 byte offsets, and rechecks project/source identity
when the selection dialog confirms insertion.

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
validates the complete final PNG frame, enforces encoded and decoded byte
budgets, and creates a collision-resistant project-relative name with a
create-only atomic link under `img/`, so an asset collision cannot replace an
existing file. Missing or invalid asset directories fail before any write.
After storage succeeds, the adapter formats the generated safe relative path
as a Typst `#image("...")` expression and delegates insertion to the focused
editor boundary; no-focused-editor outcomes leave the stored asset untouched.

Authoring services are trait boundaries so formatting and completion can run in
platform workers rather than the UI thread. Literal find/replace creates a new
result string on confirmation and intentionally performs no allocation when a
replacement is cancelled; large replacements may still create temporary peak
memory proportional to the document and replacement size.

Completion cancellation is checked before and after a provider call, so a late
result cannot be applied after cancellation even when the provider itself cannot
be interrupted. Regression tests cover this stale-result boundary and formatter
failure preservation.

CI runs formatting, clippy, and headless tests as separate jobs with the pinned
test image and read-only repository permissions. Parallel jobs shorten feedback
time but may duplicate source compilation inside their containers. AppImage
packaging is kept in a separate, manual-only Ubuntu 22.04 workflow so normal
test feedback does not download packaging tools or spend time assembling a
release image. The package workflow records tool digests and uploads the
resulting artifact; the GTK dependency tree and squashfs assembly remain the
dominant packaging I/O costs.

The CI quality gates intentionally fail on formatting drift and clippy warnings,
so the pinned toolchain and committed lockfile are part of the reproducible build
boundary rather than optional local conventions.

## CI build environments

CI uses two independent, x86_64 Ubuntu 22.04 container images: a lean test image
for Rust formatting, linting, and headless tests, and a build image that adds
AppImage and packaging dependencies. They are published manually through the CI
image workflow and consumed through repository variables containing complete
digest-qualified GHCR references. This keeps the test path independent from
release packaging while making the toolchain and GTK versions reproducible.

Image publication validates architecture, health checks, secret scans, and input
metadata before the write-enabled publication job. Consumers verify the resolved
digest and can fall back to a pinned Ubuntu runner setup when an image is missing
or unavailable. The fallback preserves availability but intentionally restores
the package-install latency that the images are meant to remove. Large GTK and
Rust layers make cold pulls and image publication the main CI infrastructure
bottleneck; role separation, immutable references, and per-role layer caches keep
that cost out of ordinary test jobs as far as possible.
