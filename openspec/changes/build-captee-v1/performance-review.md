# Per-task performance review log

This log records focused reviews of the code changed by each implementation
task. Each entry identifies the likely bottleneck, its impact, the mitigation,
and any follow-up. It is part of the change record and must be complete before
the change is archived.

## Task 4.1 — revision-aware source documents

- **Scope reviewed:** `crates/captee-core/src/editor.rs`
- **Finding:** Undo and redo currently retain complete `String` snapshots. Memory can grow approximately with document size multiplied by history depth.
- **Impact:** Large documents or long editing sessions may increase allocation and copying costs during edits, undo, and redo.
- **Mitigation:** Accepted for the initial MVP because it is simple and deterministic; the limitation is documented in `docs/architecture.md`.
- **Follow-up:** Replace snapshots with bounded edit records, coalesce continuous typing, and add a byte budget before optimizing for large documents.

## Task 4.2 — diagnostic parsing

- **Scope reviewed:** `crates/captee-core/src/diagnostics.rs`
- **Finding:** `parse_diagnostics` collects all accepted diagnostics and allocates owned path/message strings. Each line is scanned for a location in linear time.
- **Impact:** Normal compiler output is inexpensive, but pathological or repetitive output can increase memory use and downstream UI rendering work.
- **Mitigation:** The parser is linear and skips unrecognized lines; the architecture records a future cap and incremental parsing direction. The current task keeps the complete diagnostic list so callers can present deterministic results.
- **Follow-up:** Add a maximum displayed-diagnostic count and streaming adapter when compiler output limits and UI requirements are finalized.

## Task 4.3 — debounced revision scheduling

- **Scope reviewed:** `crates/captee-core/src/revision.rs`
- **Finding:** Each submission owns a complete source `String` until it is replaced or consumed. Rapid edits can therefore allocate and copy large snapshots, even though only the newest pending work is retained.
- **Impact:** Typing in large documents can create short-lived allocation pressure; the scheduler itself does not allow an unbounded pending queue.
- **Mitigation:** Coalesce pending work to one latest snapshot, debounce execution, and reject stale worker results by revision.
- **Follow-up:** Share or structurally edit document snapshots when large-document performance work replaces the current editor history model.

## Task 4.4 — authoring services

- **Scope reviewed:** `crates/captee-core/src/authoring.rs`
- **Finding:** Literal replacement allocates a complete result string and the completion trait returns an unbounded vector supplied by its provider.
- **Impact:** Replacing large documents can temporarily require source plus result memory; an overly broad completion provider could increase UI rendering and allocation work.
- **Mitigation:** Replacement is performed only after explicit confirmation, cancellation returns without changing or allocating source text, and formatter/completion work is exposed as a trait for off-thread adapters.
- **Follow-up:** Add scoped/streaming replacement and a maximum completion count when the UI provider contract is implemented.

## Task 4.5 — authoring regressions

- **Scope reviewed:** `crates/captee-core/tests/authoring_regressions.rs` and the completion cancellation guard in `src/authoring.rs`.
- **Finding:** Regression tests are small and deterministic; completion providers remain responsible for their own runtime and candidate volume because the trait call is synchronous.
- **Impact:** A slow provider can still delay the caller before the post-call cancellation check, although its result will not be applied after cancellation.
- **Mitigation:** Cancellation is checked before and after provider execution, stale scheduler results are rejected, and tests lock in failure-preservation and confirmed-replacement behavior.
- **Follow-up:** Move provider execution to a bounded worker with a maximum completion count when the UI integration is implemented.

## Task 5.1 — revision-aware render state

- **Scope reviewed:** `crates/captee-core/src/render.rs`
- **Finding:** A successful preview owns its rendered PDF bytes, while the current
  diagnostics list owns compiler messages and source paths. Applying a result is
  constant-time apart from replacing those bounded-by-input buffers; changing a
  source revision does not copy the retained preview.
- **Impact:** Keeping the last successful render makes preview recovery reliable,
  but a large PDF and a large diagnostic output remain resident until the next
  successful render or source revision. Stale results could also overwrite valid
  state if they were accepted without a revision check.
- **Mitigation:** The state accepts only the exact current revision, clears old
  diagnostics and the current-attempt timestamp when a newer source revision is
  announced, retains the prior preview on failure, and stores timestamps supplied
  by the caller without spawning threads or performing I/O.
- **Follow-up:** Task 5.2 should keep compiler work off the UI thread and pass
  bounded render outputs into this state; task 5.4 should add fixture coverage
  for the adapter boundary and stale render application.

## Task 5.2 — asynchronous preview compilation

- **Scope reviewed:** `crates/captee-platform/src/typst.rs` and the supporting
  test-isolation change in `crates/captee-platform/src/atomic.rs`.
- **Finding:** Each submitted preview owns one source snapshot and one worker
  thread until the bundled compiler exits. The adapter also materializes one
  temporary source file and one PDF before reading the PDF into the outcome.
- **Impact:** Rapid submissions can temporarily consume compiler processes,
  worker stacks, source/PDF memory, and project-directory I/O. A compiler that
  hangs would keep its worker and child process alive because the standard
  command adapter has no cancellation boundary yet.
- **Mitigation:** The existing debounced scheduler is the intended submission
  boundary, each temporary path is unique, stale outcomes are rejected by core
  render state, and every completion path removes both temporary files. The
  worker exposes a narrow trait for test doubles and keeps process details out
  of the UI.
- **Follow-up:** Add bounded process timeouts/cancellation and a latest-only
  worker queue when preview controls and task 7 UI cancellation are introduced.

## Task 5.3 — atomic PDF export

- **Scope reviewed:** `crates/captee-platform/src/export.rs` and the relative
  destination handling in `crates/captee-platform/src/atomic.rs`.
- **Finding:** Export copies the retained PDF bytes into one temporary file,
  flushes it, and atomically renames it. The destination is checked for a valid
  parent and file type before that allocation and write begin.
- **Impact:** Peak memory is approximately the retained preview plus the
  temporary PDF contents, and the export performs one full write plus directory
  synchronization. A stale or missing preview could otherwise cause an
  unintended export of an older document.
- **Mitigation:** Revision and preview-availability checks happen before any
  destination mutation, and atomic-write cleanup preserves the previous file
  when staging or replacement fails. Tests cover successful export, refusal of
  missing/stale renders, and invalid destinations.
- **Follow-up:** Add an injectable filesystem failure double or platform-level
  fault-injection test when the export fixture suite is expanded in task 5.4.

## Task 5.4 — preview and export fixture regressions

- **Scope reviewed:** `crates/captee-platform/tests/preview_export.rs` and the
  two small Typst source fixtures.
- **Finding:** The tests use bounded in-memory fixture strings and PDF bytes,
  with one temporary directory for export assertions. They do not start a
  compiler or leave worker threads running after each received outcome.
- **Impact:** Fixture coverage adds negligible runtime and memory cost; unique
  temporary roots avoid interference between parallel tests. The test double
  intentionally does not validate the real Typst binary's evolving diagnostics.
- **Mitigation:** The production adapter remains covered by its trait boundary,
  while these tests lock the revision and destination-preservation contracts
  independently of a desktop session or installed compiler.
- **Follow-up:** Add pinned real-compiler golden fixtures when the bundled
  Typst executable is included in CI and task 8.3 runs release validation.

## Task 6.1 — capture and insertion interfaces

- **Scope reviewed:** `crates/captee-core/src/capture.rs` and its public exports.
- **Finding:** The interfaces only pass owned image bytes, small annotation
  values, and insertion strings. They perform no image decoding, filesystem or
  process I/O, and do not allocate beyond the explicit owned byte boundaries.
- **Impact:** A caller can retain both captured and annotated byte buffers while
  an annotation is awaiting confirmation, so peak memory is proportional to the
  capture plus the proposed result. Adapter implementations control any larger
  rendering cost behind the trait boundary.
- **Mitigation:** Core keeps the original capture immutable by taking shared
  references, makes cancellation and failures explicit in each outcome, and
  exposes no platform resources or worker lifetimes. Later adapters can bound
  image dimensions and subprocess work without changing these contracts.
- **Follow-up:** Task 6.2 should add bounded backend execution; task 6.3 should
  preserve these cancellation/no-mutation semantics while implementing drawing.

## Task 6.2 — portal-first and bounded fallback capture

- **Scope reviewed:** `crates/captee-platform/src/capture.rs` and its public
  adapter exports.
- **Finding:** Each capture attempt starts at most one `slurp` process followed
  by one `grim` process, polls at a five-millisecond cadence, and owns their
  captured stdout/stderr until completion. The selector does not queue or spawn
  fallback work after portal success or cancellation.
- **Impact:** A large raw PNG is held in memory once by the process output and
  once by `CapturedImage`; a hung subprocess retains one worker-side child until
  the configured timeout. Polling adds small CPU wakeups during selection.
- **Mitigation:** The timeout kills and waits for a child, portal cancellation
  avoids fallback mutation, and the selector has no unbounded queue. PNG size
  and image validity remain explicit follow-ups for task 6.4.
- **Follow-up:** Add process-group cancellation where available and validate
  bounded PNG dimensions before persisting capture output.

## Task 6.3 — in-memory image annotations

- **Scope reviewed:** `crates/captee-platform/src/capture.rs` and the PNG
  dependency used by `PngAnnotationBackend`.
- **Finding:** Each annotation decodes the complete PNG into an RGBA buffer and
  encodes a complete staged PNG, so the original bytes and the proposed result
  are resident together while confirmation is pending. Rectangle and pointer
  work is bounded by the selected geometry; bitmap text work is bounded by the
  text length and fixed glyph size.
- **Impact:** Peak memory is approximately the captured PNG, decoded RGBA
  pixels, and encoded result. Large or malicious dimensions could otherwise
  cause excessive allocation or decompression work, and repeated annotations
  repeat the full-image conversion.
- **Mitigation:** Pixel-size arithmetic is checked and the decoder rejects
  images above 16 million pixels before they are accepted as an annotation
  surface. Drawing clips every pixel write to the image bounds, and the
  original `CapturedImage` is borrowed immutably so cancellation or a failed
  encode cannot mutate it.
- **Follow-up:** Task 6.4 should validate the final PNG and asset byte budget
  again at the persistence boundary, and the UI can coalesce multiple pending
  strokes before re-rendering when interactive annotation controls are added.

## Task 6.4 — validated atomic asset storage

- **Scope reviewed:** `crates/captee-platform/src/assets.rs`, the create-only
  path in `crates/captee-platform/src/atomic.rs`, and their public exports.
- **Finding:** Asset storage validates the encoded PNG size and decodes one
  complete frame before writing. It creates at most one temporary file and one
  destination link per name attempt; name generation is process-local and
  monotonic, while the destination link makes collision handling race-safe.
- **Impact:** Validation temporarily holds the annotated bytes and decoded PNG
  frame, with a bounded 32 MiB encoded asset and 64 MiB decoded frame. A
  concurrent or pre-existing name collision retries up to 128 times, and a
  failed write can perform small temporary-file and directory-sync I/O.
- **Mitigation:** Pixel, decoded-buffer, and encoded-byte limits reject large
  or decompression-heavy assets before persistence. `atomic_create` flushes and
  syncs the temporary file, uses a non-replacing hard link, removes the
  temporary link, and syncs the directory; all error paths remove leftovers.
- **Follow-up:** Task 6.5 should consume `SavedAsset::relative_path` for
  insertion without rereading or copying the image bytes.

## Task 6.5 — Typst image-expression insertion

- **Scope reviewed:** `SavedAsset::typst_image_expression`,
  `insert_saved_asset`, and the `EditorInserter` boundary in
  `crates/captee-platform/src/assets.rs`.
- **Finding:** Insertion formats one short expression and delegates one call to
  the focused editor. It does not reread the PNG, perform filesystem I/O, or
  create a worker or retained platform resource.
- **Impact:** The expression allocates a small string proportional to the
  generated relative path. A caller without focus receives an immediate typed
  outcome and the stored asset remains available.
- **Mitigation:** Only `SavedAsset` values produced by the validated asset store
  expose insertion, the path is project-relative and generated from safe fixed
  components, and the no-editor branch returns before touching editor state.
- **Follow-up:** Task 6.6 should retain regression coverage for exact insertion
  formatting alongside cancellation, malformed PNG, and atomic cleanup cases.

## Task 6.6 — capture and asset regression tests

- **Scope reviewed:** `crates/captee-platform/tests/capture_regressions.rs` and
  the atomic-create cleanup tests in `crates/captee-platform/src/atomic.rs`.
- **Finding:** The regressions use small in-memory PNG fixtures and test doubles;
  they do not launch compositor processes, allocate unbounded buffers, or retain
  background workers. Temporary project roots are isolated per test.
- **Impact:** Test runtime and memory are bounded by tiny fixture images and
  short-lived temporary directories. Parallel tests can perform filesystem I/O
  concurrently, but each uses a unique nanosecond-stamped root.
- **Mitigation:** Capture cancellation, malformed output, insertion formatting,
  and temporary-file cleanup are asserted through stable public contracts. No
  real desktop session or external capture executable is required.
- **Follow-up:** Keep process timeout and PNG size behavior covered by focused
  adapter tests when portal integration and packaging fixtures are expanded.

## Task 7.1 — UI-agnostic application state store

- **Scope reviewed:** `crates/captee-core/src/app.rs` and its public exports.
- **Finding:** Dispatch is constant-time and retains only one current snapshot;
  cloned snapshots copy project/settings and user-visible messages for the UI
  boundary. No command starts a thread, process, or filesystem operation.
- **Impact:** A large status message or settings snapshot can briefly allocate
  when a UI subscriber requests `snapshot()`, but application state remains
  bounded by the current project context and operation message.
- **Mitigation:** Typed transition guards reject busy and project-less actions
  before mutation, cancellation only clears activity, and a monotonic version
  lets adapters identify changed snapshots. Platform work remains outside the
  store and is represented by typed activity commands.
- **Follow-up:** Task 7.2 should subscribe GTK widgets to snapshots without
  retaining duplicate widget-owned project or process state; task 7.4 should
  connect cancellation controls to the same dispatcher boundary.

## Tasks 7.2–7.5 — GTK desktop presentation adapter and UI-state coverage

- **Scope reviewed:** `crates/captee-ui/src/lib.rs`, `src/main.rs`, and the
  workspace/UI-state CI jobs.
- **Finding:** The adapter performs constant-time intent routing and retains
  only the current progress and accessibility announcement. Settings validation
  is bounded to scalar checks; the GTK shell retains one widget tree and one
  source buffer for the active window.
- **Impact:** Each state snapshot clones core state plus at most one progress
  label and one announcement. GTK retains source text in the editor buffer and
  renders a PDF placeholder until the preview adapter is connected.
- **Mitigation:** Logical panes, focus, keyboard actions, progress, and status
  announcements are represented as typed values; failures clear progress and
  invalid settings do not mutate prior project settings. GTK actions dispatch
  through the same UI shell boundary, and CI installs GTK/GtkSourceView before
  workspace and UI checks.
- **Follow-up:** The AppImage job still needs a pinned packaging toolchain and
  bundled GTK runtime; native widget memory and redraw costs should be profiled
  when the preview and editor content pipelines are connected.

## Task 2.4 — core CI checks (partial)

- **Scope reviewed:** `.github/workflows/rust-checks.yml`
- **Finding:** Formatting, clippy, and core tests run as parallel jobs, each with a separate toolchain/cache setup. This improves feedback latency but can duplicate dependency compilation and consume more concurrent runner minutes.
- **Impact:** Initial CI runs may be slower and use more cache storage than a single combined job; clippy over all targets is intentionally stricter than the current headless test job.
- **Mitigation:** Pin the toolchain, use a shared cache key through `Swatinem/rust-cache`, keep permissions read-only, and scope tests to `captee-core` until platform/UI dependencies are available.
- **Follow-up:** Add an AppImage packaging job and evaluate a combined or dependency-prebuild job after GTK packaging exists. Task 2.4 remains open until that job is implemented.

### CI failure review and repair

- **Scope reviewed:** the clippy/fmt failures in the task 2.4 implementation and the affected Rust sources.
- **Finding:** Strict clippy exposed an unused binding, a manually implemented derivable default, and a rewritable `let-else`; rustfmt exposed formatting drift across the scaffold.
- **Impact:** The CI gate correctly prevented a non-reproducible quality baseline from landing, but formatting every workspace source creates a broad mechanical diff during the first cleanup.
- **Mitigation:** Applied the clippy suggestions, formatted the workspace with the pinned Rust 1.97.1 toolchain, and committed `Cargo.lock`; local clippy, rustfmt, and all 20 core tests now pass.
- **Follow-up:** Keep the strict gates and review future formatting-only diffs separately from behavioral changes.
