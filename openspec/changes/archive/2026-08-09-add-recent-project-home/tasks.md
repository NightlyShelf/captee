## 1. Recent-project data

- [x] 1.1 Persist name, path, last-access time, and pin state; keep five displayed projects, pinned first then recent.
- [x] 1.2 Support opening, removing from the list, and safe disk deletion without unintended mutation.
- [x] 1.3 Document the recent-project configuration location in README.

## 2. Home panel

- [x] 2.1 Add centered recent-project panel with New Project and Open Existing buttons in existing visual style.
- [x] 2.2 Show each project name, path, last access, pin action, and delete action; names open projects.
- [x] 2.3 Keep panel non-scrollable and accessible.

## 3. Delete confirmation

- [x] 3.1 Add dialog: Remove from list, Delete from disk, or Cancel.

## 4. Validation

- [x] 4.1 Add focused tests for ordering, limit, pinning, opening, and all delete choices.

## 5. Refinements

- [x] 5.1 Show a filled pin for pinned projects; remove missing projects from the list when disk deletion is chosen.
- [x] 5.2 Make recovery-dialog buttons smaller and properly placed.
- [x] 5.3 Highlight the selected project-tree file and refresh preview when another Typst file opens.
- [x] 5.4 Add focused regression tests.

## 6. Final fixes

- [x] 6.1 Keep the pin icon shape; show pinned state with white fill.
- [x] 6.2 Hide temporary preview files from the project tree.
- [x] 6.3 Make dialog buttons compact and match Captee style.
- [x] 6.4 Add focused regression tests.

## 7. Feedback fixes

- [x] 7.1 Show an actual white-filled pin state with the same pin shape.
- [x] 7.2 Let Typst files and folders drag before click actions run.
- [x] 7.3 Add focused regression tests.

## 8. Selection fix

- [x] 8.1 Highlight the file currently loaded by the editor.
- [x] 8.2 Add focused regression test.
