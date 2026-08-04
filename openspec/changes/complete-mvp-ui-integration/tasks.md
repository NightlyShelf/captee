## 1. Integration foundation

- [x] 1.1 Define the UI operation coordinator, active project/source revision identity, result channel, and cancellation/lifetime handling without moving platform effects into core.
- [x] 1.2 Add test doubles and focused integration tests for successful, cancelled, failed, and stale operation results.

## 2. Editor and project persistence

- [x] 2.1 Bridge GtkSourceView changes to the core source document, dirty state, undo/redo, and active entry document.
- [x] 2.2 Wire Save, atomic autosave, recovery prompt, recent-project recording, and safe project persistence through platform boundaries.
- [x] 2.3 Wire Format, literal Find/Replace, Completion, and diagnostics to the editor with revision-safe result handling.

## 3. Preview and export

- [x] 3.1 Connect asynchronous Typst preview compilation to source revisions, preview rendering, diagnostics, and stale-result rejection.
- [x] 3.2 Connect current-preview validation and native PDF destination selection to atomic PDF export.
- [x] 3.3 Add preview/export success, failure, cancellation, and last-valid-preview regression coverage.

## 4. Capture and annotation

- [x] 4.1 Add concrete portal/fallback capture adapters for the supported Linux desktop path and wire Capture cancellation/error completion.
- [x] 4.2 Build the staged annotation UI for pointer, rectangle, and text marks without mutating the original capture.
- [x] 4.3 Connect confirmation to validated PNG asset storage and focused-editor Typst insertion, with no mutation on cancellation or failure.
- [x] 4.4 Add capture, annotation, storage, insertion, fallback, and cancellation integration tests.

## 5. Settings and operation feedback

- [x] 5.1 Add settings UI and persistence for formatting, capture fallback, preview preferences, and keybindings using existing validation.
- [ ] 5.2 Add visible/accessibility-aware progress, cancellation, success, warning, and error states and disable conflicting destructive actions.

## 6. Verification and performance review

- [ ] 6.1 Review each changed integration slice for CPU, memory, filesystem/process I/O, concurrency, and resource-lifetime risks; record findings in performance-review.md and mirror architectural consequences in docs/architecture.md.
- [ ] 6.2 Run formatting, clippy, the complete test suite, OpenSpec validation, and a local Hyprland/Wayland end-to-end smoke test for editing, save, preview, capture, export, and recovery.
