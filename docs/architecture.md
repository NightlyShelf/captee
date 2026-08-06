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

Preview requests are debounced for 600 ms and use `AsyncPreviewCompiler` with
the discovered Typst binary outside the GTK thread. A successful attempt
produces the complete PDF plus a first-page PNG, both tagged with the active
source revision. `RenderState` remains authoritative for stale rejection and
last-valid PDF retention; GTK decodes the PNG only after both coordinator and
render-state checks accept it. Failed renders update diagnostics and status but
leave the last valid preview picture visible.

PDF export is available only when `RenderState` holds a successful preview for
the current source revision. A GTK native save chooser gathers a local
destination without mutation; the worker then revalidates the preview and
destination and writes the PDF through atomic replacement. Export owns a cloned
immutable render snapshot, so source edits cannot change bytes already being
written and stale completion cannot update the new revision's UI state.

Headless preview/export integration coverage crosses the UI operation channel,
platform preview outcome, core render state, and atomic export boundary. It
locks in success, failed-render retention, stale and cancelled result rejection,
and no write after destination cancellation without requiring GTK or Typst.

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
terminated and reaped on expiry. Their stdout and stderr are drained concurrently
into bounded buffers while the child runs, avoiding a pipe-capacity deadlock on
normal screenshot sizes; both reader threads are joined at the process boundary.
Output is still unvalidated raw image data until the PNG-validation task.

The concrete Linux portal adapter uses the XDG Screenshot D-Bus interface on a
worker thread and accepts only local file URIs. Portal reads are capped at 64
MiB and portal cancellation is terminal, while a genuine portal failure may
enter the configured `slurp`/`grim` path used by Hyprland. Capture results carry
the active project and source revision through the UI operation coordinator;
late results after cancellation or project replacement are discarded, and only
an accepted result becomes the single staged capture owned by the workspace.

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

The GTK annotation dialog owns a reversible `AnnotationDraft` containing an
immutable original and one staged encoded image. Pointer, rectangle, and text
controls submit one mark at a time to a worker; controls are temporarily
disabled and the accepted PNG is decoded on the GTK thread only for display.
Reset restores the original, while closing the dialog drops both buffers. No
filesystem or source mutation is reachable until the separate confirmation
boundary accepts the staged image.

Confirmation transfers the staged PNG to the platform asset store on a worker.
That boundary validates the complete image before a create-only atomic write,
then returns a safe project-relative path. Only an accepted result is handed to
the focused-editor insertion adapter, which inserts the exact generated Typst
expression at the cursor as one undoable edit. The write is non-cancellable
after confirmation so UI cancellation cannot race a committed file; cancellation
before confirmation remains a strict no-op.

Capture integration tests compose the same public coordinator, selector,
annotation draft/backend, asset store, and editor insertion boundaries used by
GTK. They cover portal success, fallback after portal failure, cancellation,
invalid image rejection, immutable staging, atomic storage, and exact undoable
Typst insertion without requiring a compositor in headless CI.

Capture backend order is desktop-aware at the platform boundary. Portal-first
selection remains the default, while a Hyprland desktop token makes the bounded
`slurp`/`grim` region path run first when that configured backend is enabled.
Cancellation remains terminal and never starts the second backend. This avoids
depending on a Hyprland screenshot portal request that may remain pending
without presenting a usable region selector, while preserving portal behavior
for other Wayland desktops.

Project settings remain part of `.captee.json`. GTK edits a detached settings
copy, validates ranges, enabled capture paths, and unique parseable accelerator
strings, then asks the platform workspace boundary to atomically replace the
validated config. Only a successful worker result updates core state and GTK
accelerators. Capture selection and auto-preview read the current core snapshot,
preview zoom changes the retained picture request, and format-on-save runs the
formatter before the same atomic document save. Older configs receive default
keybindings without migration, and closing a workspace restores application
defaults.

Operation feedback is derived from the same immutable UI snapshot as command
dispatch. A small interaction-state projection controls the workspace action
sensitivity and editor availability, while GTK presents a spinner, textual
status, and a Cancel button only for cancellable operations. Cancellation
removes coordinator ownership immediately and flips the worker token; late
results cannot mutate the UI. Non-cancellable atomic writes keep project and
editor actions disabled until their single terminal result, preventing close,
project replacement, or source edits from racing committed state.

The workspace navigation boundary now exposes a project-relative tree model to
GTK. The tree renders files and folders in parent-before-child order with
indentation and type icons, starts at roughly one sixth of the default window
width, and leaves the remaining paned area to the editor and preview. Clicks
open files, folder clicks toggle expansion, triple-click opens rename, and drag
sources/drop targets support validated moves including the project root.
Context actions route create, inline rename, move, and delete through the
platform workspace boundary after small confirmation dialogs. Tree refresh
currently scans synchronously and skips symlinks; lazy expansion and
worker-backed enumeration remain follow-up work for very large projects. The
project divider is user-resizable, the initial editor/preview divider is set to
an equal split, and long labels use GTK ellipsization.

Capture confirmation is a staged document-composition popup drawn as a small
borderless floating window on the workspace that was active when capture
began, not as a monitor-sized surface and not inside the main Captee workspace
window. The Hyprland adapter records that placement before the selector takes
focus, then moves and focuses the mapped popup at the selected screen
coordinates above the original active application. The live desktop remains
unchanged behind the popup; no selection stroke, desktop screenshot, or dim
layer is retained after the selector closes. The popup shows a short source
context while the user edits Typst
annotation code, chooses before/after placement, invokes keyboard command
suggestions, modifies the selection, or confirms/discards with Enter/Escape.
The review does not add a duplicate captured-image background; only the
popup panel is added. Only confirmation transfers image bytes
and insertion metadata to the existing bounded storage worker. Both the main
source editor and the capture editor load the checked-in Typst GtkSourceView
language definition, with a Markdown fallback for runtimes that do not package
the definition.

Capture can also be initiated without workspace focus through the XDG
GlobalShortcuts portal. The listener is owned by a platform worker and forwards
activation events to the GTK main context, where the existing capture
coordinator applies project and cancellation checks. Before creating the
session, the worker registers `com.nightlyshelf.captee` as the host app on the
same D-Bus connection, which gives unsandboxed development and installed
launches the application identity required by the portal. For a host-launched
development binary, it also installs the checked-in desktop entry into the
user application-data directory when absent, because the portal requires the
registered ID to match a desktop-entry basename.
On Hyprland, the worker additionally installs the runtime `global` keybind
that maps the requested trigger to `com.nightlyshelf.captee:capture`; XDPH
exposes the registered shortcut but does not create that compositor keybind
itself.

The review popup uses the fallback compositor coordinates directly for its
Hyprland placement and does not render a captured desktop background. Portal
captures without geometry use the active application monitor as a fallback and
show the unavailable-geometry status. GTK result polling releases the
operation coordinator's dynamic borrow before
calling any result handler. The same rule applies to shell dispatch results
that trigger follow-up label, settings, or project-lifetime reads. Capture
confirmation closes the temporary review popup before handing the staged image
to storage, so cancellation, modification, and confirmation cannot leave a
stale review surface or re-enter through a duplicate review.

Project creation writes a minimal valid Typst heading (`= Captee`) through the
same atomic workspace boundary as the config. This guarantees a new workspace
can produce its first preview immediately instead of entering an error state
before the first user edit.

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
