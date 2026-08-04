# Performance review

## Task 1.1: UI operation coordinator

- Finding: Project, revision, and operation identity checks are constant-time and retain only one active-operation record. Each completed worker sends one result through an unbounded standard-library channel.
- Impact: CPU and steady-state memory overhead are negligible for one in-flight operation. Results can remain queued briefly if the GTK adapter does not poll, but the single-use task handle and one-active-operation rule prevent an individual worker from flooding the queue.
- I/O and concurrency: The coordinator performs no filesystem, process, or GTK work. Cooperative cancellation uses an atomic token, and result polling is non-blocking. Project switches, source-revision changes, explicit cancellation, and coordinator drop prevent late work from being accepted.
- Mitigation: Every result carries project generation, source revision, and operation identity. Stale results are discarded before they can mutate UI state. Worker adapters must check cancellation around blocking boundaries and terminate owned subprocesses where supported.
- Follow-up: Task 1.2 will exercise completion, cancellation, failure, and stale-result delivery with worker doubles. Later GTK integration must poll the channel from the main context without a busy loop and must not synchronously join long-running workers.

## Task 1.2: Operation result integration tests

- Finding: The worker double sends one terminal result synchronously and uses no threads, timers, filesystem access, or subprocesses.
- Impact: There is no production runtime cost. Test runtime and memory remain constant per scenario, with six small coordinator instances and one result each.
- Mitigation: Integration tests exercise the public UI boundary rather than GTK widgets, so cancellation and stale-result regressions remain deterministic and runnable in headless CI.
- Follow-up: Platform-specific task suites will reuse the same result contract while separately testing real worker-thread and process lifetime behavior.

## Task 2.1: GTK editor bridge

- Finding: Each GtkSourceView change currently copies the complete buffer into `SourceDocument`, whose undo/redo implementation stores complete text snapshots.
- Impact: CPU and memory per edit are linear in document size, and long editing sessions can retain many full snapshots. This is acceptable for the MVP's ordinary note-sized documents but is the main scaling risk in this slice.
- I/O and concurrency: Editing, undo, and redo perform no filesystem or process I/O. Programmatic buffer updates are guarded to prevent recursive change signals, and every accepted change advances the coordinator revision before asynchronous results can apply.
- Mitigation: Identical buffer snapshots are ignored, only the active entry document is retained, and stale operation results are cancelled/rejected through the coordinator. Existing architecture follow-up retains the bounded edit-record optimization for larger documents.
- Follow-up: Persistence task 2.2 must clear dirty state only after atomic success and must avoid treating programmatic recovery/save synchronization as a new user edit.

## Task 2.2: Save, autosave, recovery, and recent projects

- Finding: Manual saves, debounced autosaves, and recent-project persistence run on named worker threads and use flushed same-directory atomic replacement. Autosave snapshots copy the current source once after 750 ms of inactivity.
- Impact: GTK remains responsive during filesystem synchronization. At most one debounce callback per edit may remain scheduled until its cheap sequence check, while only the newest callback starts I/O. Save and autosave can briefly hold one additional full source snapshot.
- Concurrency and lifetime: Save results use project/revision-tagged coordinator delivery. Autosave and recent-project results carry project identity and are ignored after project replacement. The weak window reference stops polling after window destruction; detached workers own no widgets and terminate after bounded filesystem work.
- Mitigation: Project paths are resolved through `ProjectPaths`, dirty state clears only after the core document's atomic save succeeds, successful save removes the project-local autosave, malformed recovery data never replaces source, and recovery requires explicit confirmation.
- Follow-up: If very large files make one-thread-per-debounce I/O observable, replace detached persistence workers with a bounded single-worker queue and cancel superseded autosaves before copying source.

## Task 2.3: Authoring actions and diagnostics

- Finding: Formatting stages one source snapshot in the project, runs Typst on a named worker, reads the formatted result, and removes the temporary file. Completion scans a short static candidate list; literal replacement and completion insertion create one core undo snapshot.
- Impact: Formatter process startup and whole-source copies dominate this slice. Completion is constant-space apart from returned items, diagnostics display is capped at 20 entries, and GTK remains responsive while formatting/completion workers run.
- Concurrency and lifetime: Format and completion carry project/revision operation identities and check cooperative cancellation before applying. Completion dialogs recheck source identity before insertion; formatter failure and dialog cancellation do not mutate source.
- Mitigation: Temporary formatter files use collision-resistant names and are removed on all normal result paths, stale results are discarded, completion validates UTF-8 byte offsets, and formatting diagnostics retain severity/location without replacing editable source on failure.
- Follow-up: Subprocess cancellation currently prevents result application but cannot interrupt an already-running Typst formatter; a later process supervisor should terminate the child when cancellation latency becomes observable.

## Task 3.1: Connected Typst preview

- Finding: One debounced preview request copies the active source, starts the asynchronous compiler, and asks bundled Typst for both the complete PDF and a first-page PNG. `RenderState` retains one PDF while GTK retains one decoded preview texture.
- Impact: Typst process startup and dual compilation dominate CPU and I/O; memory peaks include source, PDF, PNG, and decoded texture. The 600 ms sequence debounce prevents compilation on every keystroke, and only one UI operation is accepted at a time.
- Concurrency and lifetime: Compilation runs outside GTK, results carry project/source identity, `RenderState` independently rejects stale revisions, and a failed current render retains the last valid PDF and picture. Window teardown stops polling and late results cannot reach widgets.
- Mitigation: Preview staging files use collision-resistant project-local names and are removed after each attempt. The UI displays only the first page, caps diagnostics, and drops superseded results before texture decoding.
- Follow-up: Large multi-page documents currently compile twice to produce PDF and PNG. A future renderer integrated as a library could rasterize the retained PDF once and avoid the second Typst process.

## Task 3.2: Current-preview PDF export

- Finding: Export clones the retained current PDF once, validates destination state on a worker, and performs one flushed atomic replacement. The native chooser itself performs no project mutation.
- Impact: Memory briefly grows by the PDF size while the worker owns its snapshot; filesystem I/O is linear in the PDF size and does not block GTK.
- Concurrency and lifetime: The export operation is tagged with the current project/source revision. A source change before worker completion makes the result stale in the coordinator, while `export_pdf` independently rejects a stale preview before writing.
- Mitigation: The destination parent and existing target type are validated before atomic write, cancellation of the chooser starts no operation, and missing/current-preview validation occurs before presenting the chooser.
- Follow-up: Very large exports could avoid the PDF clone by storing the immutable preview bytes in an `Arc<[u8]>`; retain the current simpler ownership until profiling shows a material peak.

## Task 3.3: Preview/export regression coverage

- Finding: The new integration tests use in-memory preview artifacts and isolated temporary export directories; they start no GTK session, compiler process, or long-lived worker.
- Impact: Production runtime is unchanged. Test CPU and memory are linear only in tiny fixture buffers, and temporary filesystem I/O is bounded to one export per relevant scenario.
- Mitigation: Tests cross the public UI coordinator, platform preview outcome, core render state, and atomic export boundaries, making cancellation, staleness, last-valid retention, and cancelled destination regressions deterministic in headless CI.
- Follow-up: The final Hyprland smoke test still needs to exercise the real native chooser and bundled Typst binary because headless tests intentionally do not emulate compositor dialogs.

## Task 4.1: Portal and Hyprland capture wiring

- Finding: Capture now runs on one named worker and asks the XDG Screenshot portal first through a minimal `ashpd` feature set. A portal failure may start one bounded `slurp` selection followed by one bounded `grim` process; cancellation never triggers fallback.
- Impact: GTK remains responsive while D-Bus or compositor interaction is active. A completed capture owns one encoded image buffer, capped at 64 MiB for portal files; the fallback pipe remains bounded by the 120-second process timeout but can allocate its complete standard output before later PNG validation.
- I/O and concurrency: Portal file reading and fallback process I/O stay off the GTK thread. Every worker reports exactly one completed, cancelled, or failed coordinator result. A source/project lifetime cancellation causes completed bytes to be discarded before UI mutation, and project replacement clears any staged capture.
- Mitigation: Only local `file:` portal results are read, URI escaping is decoded by the URL parser, reads stop after the encoded-byte cap, empty results fail explicitly, portal cancellation is distinguished from failure, and fallback children are killed and reaped at timeout.
- Follow-up: The portal wrapper waits for the portal response internally, so cooperative cancellation can disregard a late response but cannot yet close an already-displayed portal dialog. Task 5.2 will expose cancellation immediately at the UI boundary; a lower-level request adapter is needed if prompt closure becomes a material usability issue. Task 4.3 will validate PNG structure and decoded memory before storage.

## Task 4.2: Reversible annotation dialog

- Finding: The dialog retains one immutable encoded original and one encoded staged PNG. Each mark copies the staged encoded bytes once for a named worker, then the annotation backend allocates one bounded RGBA surface and one replacement PNG; GTK decodes only the accepted stage for display.
- Impact: Peak memory is proportional to two or three encoded PNG buffers plus a bounded decoded surface. Pointer, rectangle, and fixed-glyph text drawing are linear in the affected geometry, while PNG decode/encode is linear in image pixels. The 16-million-pixel backend limit prevents unbounded decoded allocation.
- I/O and concurrency: Annotation has no filesystem or subprocess I/O and runs outside the GTK loop. The dialog disables add, reset, and confirmation while one mark is processing, polls one single-result channel every 50 ms, and drops late results after dialog closure.
- Mitigation: Reset replaces the stage from the immutable original, failed/cancelled marks preserve the previous stage, texture decoding occurs only after worker completion, and cancellation clears both pending buffers without project or source mutation.
- Follow-up: Repeated cumulative marks re-encode the whole PNG each time. If profiling shows latency on large captures, retain an RGBA working surface for the dialog and encode only for display checkpoints and final confirmation.

## Task 4.3: Confirmed asset storage and editor insertion

- Finding: Confirmation moves one staged PNG to a named worker, where the existing asset store performs bounded full-frame validation and a create-only atomic write. The GTK-side insertion creates one source-document undo snapshot and one buffer snapshot after storage succeeds.
- Impact: Validation and storage are linear in encoded and decoded image size, bounded at 32 MiB encoded, 16 million pixels, and 64 MiB decoded. Source insertion is linear in document size under the current snapshot-based editor history and does not add another image copy after the worker result is delivered.
- I/O and concurrency: PNG decode and filesystem synchronization stay off GTK. The confirmed write is deliberately non-cancellable once started, preventing a cancellation acknowledgment from racing a committed asset. The saved asset result remains project/revision tagged; insertion runs only for an accepted result and an active focused editor.
- Mitigation: Invalid PNGs fail before file creation, unique names use create-only atomic linking, insertion is attempted only after storage success, and cancel before confirmation starts neither storage nor source edits. Missing focus preserves the successfully saved asset and reports a warning rather than inventing an insertion target.
- Follow-up: A source edit during the short non-cancellable write can make the completion stale, leaving a valid but unreferenced image in `img/`. Task 5.2 will disable conflicting editor actions during confirmed storage; a future project-scoped result identity could also report the saved path independently of source revision.

## Task 4.4: Capture pipeline integration coverage

- Finding: Four new headless scenarios drive the UI operation coordinator through portal/fallback selection, reversible annotation, validated atomic storage, and cursor insertion using small in-memory PNG fixtures and isolated temporary project roots.
- Impact: Production runtime is unchanged. Test CPU and memory are bounded by 24×16 pixel fixtures; filesystem work creates at most one asset in a scenario and each temporary root is removed synchronously.
- Concurrency and lifetime: The tests use deterministic single-result coordinator delivery and backend call counters rather than compositor or D-Bus processes, eliminating timing races while retaining operation identity and cancellation semantics.
- Mitigation: Coverage now proves the full successful portal path, portal-failure fallback, cancellation without fallback/storage/source mutation, malformed annotation/storage rejection, immutable originals, safe relative expressions, and undoable insertion.
- Follow-up: Headless doubles intentionally cannot validate the compositor-owned portal dialog or GTK focus transitions. Those remain explicit items in the final Hyprland/Wayland smoke test.

## Task 5.1: Validated persistent project settings

- Finding: The settings dialog edits one cloned project-settings value, validates numeric ranges, capture availability, unique/non-empty GTK accelerators, and accelerator syntax, then writes one complete project config through atomic replacement on a named worker.
- Impact: Validation is constant-sized (seven keybindings) and negligible. Config serialization and I/O are linear in the small JSON document. Preview zoom changes only the picture request; enabling auto-render schedules the existing debounced compiler. Format-on-save adds one formatter process and one source snapshot before the existing atomic save.
- I/O and concurrency: Settings persistence, optional save-time formatting, and document save remain off GTK. Settings become active only after atomic config success. The settings write and confirmed document save are non-cancellable, preventing UI acknowledgment from racing a committed config or source file.
- Mitigation: Failed validation or persistence leaves both in-memory and on-disk settings unchanged, legacy configs receive default keybindings through Serde defaults, project identity fields are preserved, and closing the project restores default application accelerators. Preview dimensions are capped at 8192 pixels per requested axis.
- Follow-up: Typst's bundled formatter currently has no connected line-width argument, so line width is persisted as a forward-compatible preference while format-on-save itself is applied. Repeated save-time formatting incurs process startup; retain opt-in behavior and profile before considering an in-process formatter.
