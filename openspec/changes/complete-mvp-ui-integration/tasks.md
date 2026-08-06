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
- [x] 5.2 Add visible/accessibility-aware progress, cancellation, success, warning, and error states and disable conflicting destructive actions.

## 6. Verification and performance review

- [x] 6.1 Review each changed integration slice for CPU, memory, filesystem/process I/O, concurrency, and resource-lifetime risks; record findings in performance-review.md and mirror architectural consequences in docs/architecture.md.
- [x] 6.2 Run formatting, clippy, the complete test suite, OpenSpec validation, and a local Hyprland/Wayland end-to-end smoke test for editing, save, preview, capture, export, and recovery.

## 7. Document-aware capture review

- [x] 7.1 Define staged capture-review state that retains the selected region and available geometry, keeps the selected image visible, darkens the surrounding workspace, renders the prior Typst context in an in-place overlay, and prevents source or asset mutation before confirmation.
- [x] 7.2 Add the capture annotation editor with full Typst editing, syntax highlighting, command autocomplete, default-before placement, and a Before/After toggle that updates the staged document composition.
- [x] 7.3 Add Modify/reselect behavior plus Escape discard, Enter confirmation, focus handling, and accessible labels for the review controls.
- [x] 7.4 Connect confirmed annotation placement to validated asset storage and insertion of the annotation and image Typst blocks in the selected order, with focused regression tests for cancellation and reselect.
- [x] 7.5 Review the changed capture slice for CPU, memory, I/O, concurrency, and resource lifetime; record findings in performance-review.md and mirror architectural consequences in docs/architecture.md.
- [ ] 7.6 Run the focused capture tests and an on-screen capture-review smoke check, including confirm, discard, placement toggle, and modify/reselect.

## 8. Shared Typst editor assistance

- [x] 8.1 Extract or extend a shared Typst syntax-highlighting definition and apply it consistently to the main source editor and capture annotation editor.
- [x] 8.2 Provide command autocomplete in both Typst editors with cursor-aware insertion, dismissal, stale-result handling, and focused tests.
- [x] 8.3 Review the changed editor-assistance slice for CPU, memory, I/O, concurrency, and resource lifetime; record findings in performance-review.md and mirror architectural consequences in docs/architecture.md.
- [ ] 8.4 Run focused editor tests and an on-screen smoke check for highlighting, command suggestions, acceptance, and dismissal in both editors.

## 9. Interactive project workspace

- [x] 9.1 Replace the project listing with a recursive clickable tree showing the project name, add-file/add-folder icons, files, and folders; support direct expand/collapse, inline rename, opening files, truncation, and a resizable project divider.
- [x] 9.2 Add valid drag-and-drop moves and context-menu create, move, and delete actions through project-root-safe platform boundaries, with small confirmation dialogs for every mutation.
- [x] 9.3 Style the workspace menu as compact regular menu items with no rounded button treatment or stroke, add the horizontal separator, and set the initial project/editor/preview geometry to approximately 1/6, 5/12, and 5/12 of the window width.
- [ ] 9.4 Add focused tree, drag/drop, context-menu, confirmation, path-validation, and layout tests.
- [x] 9.5 Review the changed workspace slice for CPU, memory, I/O, concurrency, and resource lifetime; record findings in performance-review.md and mirror architectural consequences in docs/architecture.md.
- [ ] 9.6 Run focused workspace tests and an on-screen smoke check for tree navigation, drag/drop, create, move, delete, cancellation, menu styling, and split layout.

## 10. Final verification

- [ ] 10.1 Run formatting, the complete workspace test suite, strict OpenSpec validation, and the complete visual workflow smoke test after all feature slices are implemented.
- [ ] 10.2 Run warning-denied Clippy only after tasks 7–9 are complete, then resolve any failures before handoff.
