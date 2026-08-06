## Why

The current GTK shell has an end-to-end MVP workflow, but the capture step still needs a document-aware review surface and the workspace needs the navigation and authoring affordances expected from a desktop IDE. The next increment makes capture placement reversible and visible, gives every Typst editor shared language assistance, and turns the project panel into an interactive tree while preserving the existing safe persistence boundaries.

## What Changes

- Connect the GTK source editor to the core document model and atomic project persistence, including dirty state and autosave/recovery.
- Execute formatting, find/replace, completion, save, and diagnostics through the existing core/platform boundaries.
- Connect asynchronous Typst preview rendering and current-revision PDF export to the workspace preview pane and dialogs.
- Connect portal/fallback capture, annotation, validated asset storage, and focused-editor Typst insertion.
- Add a post-selection capture review surface that retains the selected region, previews the insertion point with dimmed surrounding Typst content, supports full Typst annotation code, before/after placement, modify/reselect, and keyboard confirm/discard.
- Provide Typst syntax highlighting and command autocomplete in both the main editor and capture annotation editor.
- Replace the static project listing with a clickable, draggable tree, file/folder toolbar actions, confirmed context-menu mutations, and a responsive 1/6 project-panel layout.
- Use compact regular menu styling and divide the menu from the workspace; split the remaining workspace evenly between editor and preview.
- Add settings and operation-progress/cancellation UI where the existing state contracts require it.
- Add integration-focused tests and visual smoke checks for each new interaction, then run the complete final validation locally.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-workspace-ui`: visible menu actions must execute real operations and expose progress, cancellation, success, and failure.
- `typst-authoring`: the GTK source editor must use the core authoring and persistence contracts.
- `document-preview-export`: preview rendering and PDF export must be reachable from the workspace.
- `screenshot-annotation`: capture, annotation, storage, and insertion must be reachable from the workspace.
- `workspace-management`: active project persistence, autosave/recovery, recent projects, and safe project actions must be connected to the UI.

## Impact

- `crates/captee-ui` GTK integration, operation coordinator, dialogs, and tests.
- `crates/captee-platform` concrete desktop adapters and persistence bridges where required.
- Existing core contracts remain headless and gain integration tests through test doubles.
- `docs/architecture.md` and the active change performance review will document threading, I/O, and resource-lifetime consequences.
- The next implementation frontier primarily affects `crates/captee-ui`, with project-tree mutations continuing to use the existing platform path-validation and atomic/trash boundaries.
