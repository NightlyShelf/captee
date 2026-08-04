## Why

The current GTK shell displays the workspace and exposes actions, but most actions stop after announcing that an operation started. The headless core and platform adapters already define the behaviors needed for the MVP; they must now be connected through real UI workflows so editing, persistence, preview, export, capture, and feedback work end to end.

## What Changes

- Connect the GTK source editor to the core document model and atomic project persistence, including dirty state and autosave/recovery.
- Execute formatting, find/replace, completion, save, and diagnostics through the existing core/platform boundaries.
- Connect asynchronous Typst preview rendering and current-revision PDF export to the workspace preview pane and dialogs.
- Connect portal/fallback capture, annotation, validated asset storage, and focused-editor Typst insertion.
- Add settings and operation-progress/cancellation UI where the existing state contracts require it.
- Add integration-focused tests with platform doubles and verify the complete GTK workflow locally.

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
