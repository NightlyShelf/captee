## 0. Workflow rule

- [x] 0.1 Add the confirmed proposal, commit, push, review, archive, and PR workflow to AGENTS.md.

## 1. Unified review editor

- [x] 1.1 Replace separate context and annotation areas with one Typst editor: prior content gray, editable annotation at insertion point, original line numbers kept.
- [x] 1.2 Remove selection coordinates; align placeholder and clear it on first annotation input.

## 2. Capture flow

- [x] 2.1 Preserve staged annotation text, placement, and capture when Modify reselects.
- [x] 2.2 Allow empty annotation and insert only the image.
- [x] 2.3 After confirmation, focus the source at insertion and scroll preview to end.

## 3. Validation

- [x] 3.1 Add focused tests for draft preservation, empty annotation, insertion focus, and preview scroll.

## 4. Review fixes

- [x] 4.1 Rebase this change on current main so recent-project home stays available.
- [x] 4.2 Place annotation placeholder at editable insertion line.
- [x] 4.3 Require a fresh `origin/main` before each new change.
- [x] 4.4 Always center capture review window.
- [x] 4.5 Let Shift+Enter add an annotation line.
