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
