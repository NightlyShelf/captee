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

- Finding: One debounced preview request copies the active source, starts the asynchronous compiler, and asks bundled Typst for both the complete PDF and one PNG per rendered page. `RenderState` retains one PDF while GTK retains the accepted page textures in a scrollable list.
- Impact: Typst process startup and dual compilation dominate CPU and I/O; memory peaks include source, PDF, all page PNGs, and decoded textures. The 600 ms sequence debounce prevents compilation on every keystroke, and page count now scales preview memory linearly with the document.
- Concurrency and lifetime: Compilation runs outside GTK, results carry project/source identity, `RenderState` independently rejects stale revisions, and a failed current render retains the last valid PDF and page list. Source edits keep the editor interactive, cancel the superseded preview, clear UI progress, and allow the next debounced request. Window teardown stops polling and late results cannot reach widgets.
- Mitigation: Preview staging files use collision-resistant project-local names and are removed after each attempt. The UI appends pages only after all page bytes decode successfully, preserves document order, caps diagnostics, and drops superseded results before widget updates. Fit-page scaling changes only GTK size requests and does not recompile; the viewport-size notification reapplies it after layout changes and constrains both page dimensions.
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

## Task 5.2: Progress, cancellation, and interaction feedback

- Finding: One existing 100 ms GTK main-context poll now synchronizes spinner visibility, Cancel availability, editor/action sensitivity, and status styling from the immutable UI snapshot; it adds no worker, channel, filesystem, or process work.
- Impact: Each tick performs constant-time snapshot cloning and updates a fixed set of 15 actions. This is negligible beside rendering and capture work, though unchanged widget properties may still receive repeated setters while the window is open.
- Concurrency and lifetime: Cancel immediately removes the active coordinator operation and flips the cooperative token before returning the shell to idle. Late results are ignored, while project/source-lifetime stale results still report warnings. Non-cancellable writes expose no Cancel control and keep editor/project actions disabled until terminal delivery.
- Mitigation: Spinner and text provide redundant progress cues, the visible status label and dynamic tooltip expose text to assistive technology, success/warning/error CSS classes do not replace textual meaning, and a pure interaction-state function has headless coverage for home, busy/cancellable, and resumed states.
- Follow-up: The `ashpd` high-level screenshot request cannot close an already displayed portal prompt when its future is cancelled; Captee immediately becomes usable and rejects its late result, but prompt closure would require a lower-level D-Bus request implementation. Property-diffing can be added if the fixed 100 ms widget updates ever appear in profiles.

## Task 6.1: Complete integration performance and lifetime audit

- Scope reviewed: All branch changes across editor snapshots, autosave/recovery, formatting/completion, preview/export, portal/fallback capture, annotation/storage/insertion, settings persistence, GTK feedback, worker channels, subprocesses, and temporary-file cleanup.
- CPU and memory: The dominant accepted costs remain whole-document undo/source snapshots, dual Typst preview compilation, full PNG decode/encode per annotation, and retained PDF/PNG/texture buffers. Debouncing, one-active-operation coordination, diagnostic caps, 16-million-pixel PNG bounds, 32/64 MiB encoded/decoded asset limits, and 8192-pixel preview requests bound or coalesce the practical MVP load.
- Filesystem and process I/O: Source/config/PDF writes use flushed same-directory atomic replacement; capture assets use create-only atomic linking; formatter/preview staging cleans temporary files; capture and Typst work run outside GTK. The audit found and fixed a material `grim` pipe deadlock: stdout/stderr are now drained concurrently while the child runs, bounded at 64 MiB/1 MiB, and reader threads are joined after child exit or termination.
- Concurrency and lifetime: One coordinator operation, project/source identities, cooperative tokens, weak GTK/application references, single-result channels, and action/editor disabling prevent late mutation and conflicting lifecycle changes. Child capture processes are killed and reaped on timeout; pipe readers terminate and join. Annotation polling stops with its dialog, and the main poll stops with its window.
- Residual risks accepted for MVP: The high-level portal prompt cannot be programmatically closed after Captee-side cancellation; preview invokes Typst twice; source history uses complete snapshots; and one cheap timer callback is retained per debounce request until its sequence check. None can mutate stale project/source state, and each has a documented bounded or future mitigation.
- Outcome: No additional unbounded queue, unreaped owned process, GTK-thread blocking operation, or write-before-validation path was found after the pipe fix. Final validation and real Hyprland/Wayland workflow testing remain task 6.2.

## Task 6.2: Final validation and Hyprland/Wayland smoke test

- Environment and coverage: The native GTK application was exercised on Hyprland with `WAYLAND_DISPLAY=wayland-1`, 1.5 display scaling, `slurp`/`grim`, and the verified Typst 0.14.2 bundle. The smoke path covered named project creation under `/tmp`, native folder opening, editing, autosave recovery, atomic save, automatic and manual preview, native PDF export, region capture, annotation confirmation, PNG storage, exact editor insertion, and project close back to the menu-free home screen.
- Findings: The smoke test found four correctness/lifetime defects that headless adapters did not expose: the starter source used invalid Typst syntax; the GTK result poll retained a coordinator `RefMut` while applying a result; annotation confirmation re-entered its response callback through `close`; and project close retained a shell `RefMut` while refreshing the label. It also reproduced the reported Hyprland portal wait where no usable region selector appeared.
- Mitigation: New projects now start with the valid `= Captee` heading and have a regression assertion. Ready results are drained with the coordinator borrow released before callbacks, covered by a re-borrow regression test. Annotation owns its confirmed image before hiding the dialog, and close/settings dispatch results are materialized before follow-up UI reads. Hyprland is detected from `XDG_CURRENT_DESKTOP` and prefers the already bounded `slurp`/`grim` path; portal-first behavior remains unchanged elsewhere and both backend orders have focused tests.
- Performance and lifetime impact: Desktop detection is one short environment lookup and case-insensitive token scan per capture. The fallback still starts at most one `slurp` and one `grim`, retains the existing 120-second timeout and output limits, drains pipes concurrently, and reaps children. Result draining remains one non-blocking channel receive per ready result and no longer extends dynamic borrows into callbacks. Hiding the annotation dialog avoids recursive response delivery without retaining image copies beyond the existing staged/worker ownership.
- Verification outcome: Formatting, warning-denied Clippy, the complete workspace test suite, and all seven OpenSpec validation checks pass after the smoke fixes. The exported artifact was a one-page PDF 1.7/A4 document; capture produced validated PNG assets and persisted project-relative `#image(...)` expressions; recovered unsaved capture content was preserved; and closing the project left the process alive with only New/Open controls visible.
- Residual follow-up: Captee-side cancellation cannot forcibly dismiss an already-running high-level portal request, but Hyprland no longer selects that path by default when `slurp`/`grim` is enabled. The bounded fallback selector remains user-cancellable through its native selection UI and times out after 120 seconds.

## Task 7: Document-aware capture review

- Finding: The post-selection review is a small borderless floating GTK popup moved to the Hyprland workspace that was active before selection and positioned from the selected compositor coordinates. It retains no selection frame, desktop screenshot, or dim layer; the live desktop remains underneath.
- Impact: The GTK review adds one temporary top-level surface, one editable source buffer, and a short context string while visible. Placement adds one bounded active-workspace query before capture and at most 20 short window-list/dispatch retry cycles after mapping. Annotation insertion creates one additional source string proportional to the annotation and image expression, while PNG validation and storage remain linear in image size.
- I/O and concurrency: No filesystem mutation occurs when the review opens, toggles placement, edits code, or is discarded. Modify starts a new capture operation after dropping the staged capture. Enter/confirm transfers only the selected image and insertion metadata to the non-cancellable storage worker.
- Mitigation: Capture and source identity checks remain at the coordinator boundary; the selected order is represented explicitly; the review's Escape and Cancel paths clear staged state; command suggestions insert only through the editor buffer; image storage still uses the existing bounded PNG validator and create-only atomic write. Hyprland placement retries are bounded and run off the GTK thread; monitor selection and local placement run once after allocation, and the surface is closed on every review action.
- Follow-up: Portal captures still have no geometry in the portal response and use the active application monitor as a fallback while showing an explicit unavailable-geometry label. Non-Hyprland desktops retain their normal window placement because no portable workspace-address API is assumed. Fractional-scale compositor coordinate mapping is intentionally heuristic because the fallback protocol exposes raw coordinates without a monitor transform; a compositor-specific monitor adapter can replace it if a desktop reports inaccurate placement. The older pointer/rectangle/text image-mark dialog remains available as a compatibility path but is not used by the new capture insertion flow.

## Task 8: Shared Typst editor assistance

- Finding: Both the main editor and capture review use the same Typst GtkSourceView language definition, an explicit dark style scheme/CSS presentation, and bounded command-suggestion lists (the capture editor through Ctrl+Space). The main editor continues to use its existing revision-aware completion operation.
- Impact: Syntax highlighting is incremental in GtkSourceView and adds no worker or filesystem I/O. The capture suggestion list is a fixed four-item allocation and inserts only the selected command text.
- Mitigation: The language definition is loaded from the checked-in UI asset directory with Markdown fallback if a packaged runtime omits it; dismissal has no source mutation; existing completion cancellation and stale-result handling remain unchanged.
- Follow-up: The current command list is intentionally small and static for this slice. A later language-service integration can replace it with context-aware Typst documentation and cursor-range replacement without changing the editor boundary.

## Task 9: Interactive project workspace

- Finding: The project tree enumerates project-relative entries through a platform boundary, renders stable parent-before-child indentation and type icons, supports direct expand/collapse, inline triple-click rename, row activation, drag sources, folder/root drop targets, context-menu create/rename/move/delete, and compact confirmation dialogs. The initial navigation width is 212 pixels for the 1280-pixel default window, is user-resizable, and the editor/preview divider is initialized to an equal split.
- Impact: Tree refresh performs one bounded recursive directory scan on the GTK thread and rebuilds one row per project entry. This is acceptable for the MVP's small local projects but can make very large trees observable during refresh. Drag/drop and context actions add no background worker yet.
- I/O and concurrency: Create/move operations validate project-relative paths through `ProjectPaths`; move rejects self/descendant destinations and refreshes only after success. Delete confirmation routes the accepted absolute path through the desktop Gio trash boundary. Context confirmation prevents declined actions from mutating the project.
- Mitigation: Symlink entries are skipped during tree enumeration, unsafe names and path escapes are rejected, collisions fail before mutation, and all rows are rebuilt from the project root after a successful operation.
- Follow-up: Add a bounded worker-backed tree model for very large projects and persist selection/expanded folders across project reloads.

## Tasks 7.6, 8.4, and 9.4: Focused regression coverage

- Finding: Added headless capture-review, shared Typst completion, project-tree, context-action, confirmation, path-validation, deletion, and layout coverage. Completion replacement scans only the command prefix at the captured cursor; tree tests use isolated temporary roots and bounded fixtures.
- Impact: Production cost is unchanged except for one bounded prefix scan and one small completion-list rebuild when capture-editor suggestions are opened. Test CPU, memory, and filesystem I/O remain proportional to fixture source and tree size.
- Mitigation: Completion results are rejected when the operation source revision is stale, dismiss/cancel paths perform no edit, tree mutations pass through project-root validation, and every temporary test root is removed synchronously.
- Follow-up: Keep compositor-driven interaction checks in the visual smoke workflow; headless tests intentionally do not emulate GTK pointer focus or native dialogs.

## Tasks 7.6, 8.4, and 9.6: Native smoke readiness

- Finding: The GTK application starts and maps on the available Hyprland session with the expected Captee window class and initial editor/preview surface. Full pointer-driven capture, annotation, and project-tree interaction remains compositor/input dependent.
- Impact: Startup adds no new worker or persistent resource. Capture suggestions rebuild a small popover synchronously on Ctrl+Space; no subprocess or filesystem work runs on the GTK thread.
- Mitigation: Capture review state is headless and immutable until confirmation, completion edits replace only the current prefix, and stale/cancelled coordinator results remain ignored.
- Follow-up: Run the full pointer-driven confirm/discard/placement/modify and tree drag/drop smoke path in a session with a Wayland input injector or manually before release.

## Tasks 10.1 and 10.2: Final local verification

- Finding: Formatting, strict OpenSpec validation, workspace tests, and warning-denied Clippy are explicit final gates. One pre-existing large-pipe capture regression test was transiently flaky during the first parallel workspace run and passed on focused rerun.
- Impact: Verification has no production runtime impact. The large-pipe test exercises bounded subprocess output draining and can be sensitive to host scheduling.
- Mitigation: Re-run the complete suite after focused failures; retain the existing bounded reader threads, child timeout, and process reaping behavior.
- Follow-up: CI remains required for the final branch because local compositor workflow coverage cannot be fully automated here.

## Preview panel refinement

- Finding: Preview presentation now removes the diagnostics block and redundant Preview heading, adds a Fit page width mode, and hides the status bar until enabled from View. Missing Typst executables now report their resolved path and setup command instead of only `No such file or directory`.
- Impact: Fit page width performs one bounded width calculation per rendered page and status-bar toggling changes only widget visibility. Removing diagnostics widgets lowers preview layout and widget-update work. Compiler discovery adds no process or filesystem work beyond the existing failed launch.
- Mitigation: Fit page retains two-dimensional bounds, fixed scales remain unchanged, status defaults off through a named constant, and render diagnostics remain revision-safe in core state even though preview UI no longer displays them.
- Follow-up: Keep compiler bundling in packaging and make `tools/fetch-typst.sh` part of developer setup when no PATH compiler or `CAPTEE_TYPST_BINARY` exists.

## Workspace header and navigation refinement

- Finding: Moved compact menu buttons into the top header, removed the duplicate Captee title, placed project path plus Captee text after the menus, hid rendered accelerator hints from menu item labels, reduced project-tree controls, and changed initial navigation sizing from one sixth to one eighth while allowing shrink below the initial width.
- Impact: Header and tree layout now use fewer pixels and less widget spacing. Paned resizing remains constant-time; no new I/O, process, or worker lifetime is introduced.
- Mitigation: Menu action accelerators remain registered independently of visible menu labels, tree actions retain accessible tooltips, and `set_shrink_start_child(true)` removes the previous hard minimum imposed by the navigation pane.
- Follow-up: Recheck typography and minimum readable tree width on displays below 1024 pixels.

## Workspace chrome refinement

- Finding: Reduced menu typography without changing menu geometry, tightened menu gaps and button padding, centered the project name/path/Captee metadata against the full window, restored and indented the project title to the tree content, removed wide paned handles, removed the preview status label from the rendered pane, hid workspace chrome on Home, and seeded the Open chooser from recent-project state.
- Impact: The GTK header and divider chrome use fewer visible pixels; preview updates no longer perform label text layout or widget updates. No new I/O, worker, or process lifetime is introduced.
- Mitigation: Render failures and progress remain available through the global status channel, while page display and scale controls retain their existing revision checks and scroll behavior.
- Follow-up: Recheck the compact header and narrow divider handles on small displays.

## Global capture shortcut

- Finding: Capture registration now uses the XDG GlobalShortcuts portal in a named worker, with one GTK timer forwarding activation events to the existing capture coordinator.
- Impact: Startup adds one portal session/bind request and one bounded event stream thread; no GTK or filesystem work is performed by the shortcut worker. The worker remains alive for the application lifetime so another focused application can trigger selection.
- Mitigation: The worker ensures the checked-in lowercase desktop entry is present in the user application-data directory when running outside a package, then registers `com.nightlyshelf.captee` as a host app on the same D-Bus connection before creating the shortcut session. On Hyprland it also performs one bounded `hyprctl keyword bind` call to connect the requested trigger to the registered global shortcut; this is required because XDPH exposes the shortcut but does not install the compositor keybind. This avoids the portal's missing-app-info rejection and Hyprland's otherwise inert registration for unsandboxed development launches. Registration or bind failures are surfaced as status text, activation still passes through the normal project/busy/cancellation checks, and the existing capture worker remains responsible for screenshot subprocess bounds.
- Follow-up: Some desktop portals require a one-time shortcut approval/configuration flow, and portal availability varies by compositor; the in-window Capture action remains the fallback.
