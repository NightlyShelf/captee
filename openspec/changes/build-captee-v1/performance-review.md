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
