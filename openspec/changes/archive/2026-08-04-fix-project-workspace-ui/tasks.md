## 1. Project lifecycle dialogs

- [x] 1.1 Add a New Project dialog that validates a non-empty name, selects a parent location, and creates the named project directory without mutating state on cancel.
- [x] 1.2 Keep Open Project on a folder-selection dialog that loads the selected project and reports invalid-project errors in the status surface.
- [x] 1.3 Centralize successful open/create and close transitions so the workspace, home surface, editor buffer, project label, and status stay synchronized.

## 2. Workspace menu presentation

- [x] 2.1 Remove duplicate New/Open/File controls from the global header and home surface while preserving the intentional home create/open actions.
- [x] 2.2 Add compact left-aligned regular menu buttons to the workspace and keep project-only actions unavailable while home is visible.

## 3. Regression coverage and verification

- [x] 3.1 Add focused headless lifecycle and validation tests for project open/close and New Project inputs.
- [x] 3.2 Review only the changed UI/dialog code for performance, I/O, concurrency, and process-lifetime risks; record mitigations in performance-review.md and mirror architectural consequences in docs/architecture.md.
- [x] 3.3 Run formatting, linting, tests, and launch the GTK application locally to verify the project dialogs and workspace transitions before committing.
