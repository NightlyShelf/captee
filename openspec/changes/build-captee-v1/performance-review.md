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
