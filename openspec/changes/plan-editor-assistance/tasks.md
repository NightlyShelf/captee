## 1. Tinymist integration

- [x] 1.1 Pin the Tinymist 0.14.6 x86_64 Linux release matching Typst 0.14.2, verify its checksum, and bundle its binary, license, and applicable notices in the AppImage.
- [x] 1.2 Add a testable stdio JSON-RPC client that starts one Tinymist LSP process for the active project, initializes it with the project root, and stops it on project close or application exit.
- [x] 1.3 Synchronize the main source and capture-review virtual Typst documents with Tinymist, and reject responses for stale document versions.
- [x] 1.4 Keep editing, saving, formatting, preview, and export usable with a clear status message when Tinymist cannot start or exits unexpectedly.

## 2. Completion popup

- [x] 2.1 Request Tinymist completion automatically after `#` and while its command prefix changes in either Typst editor.
- [x] 2.2 Show a cursor-anchored popup with Up/Down selection, Enter/Tab or pointer acceptance, and Escape or unrelated-input dismissal.
- [x] 2.3 Apply only Tinymist's returned text edit or replacement range and preserve source when the request is stale, dismissed, or empty.
- [x] 2.4 Remove the built-in completion provider, manual completion command and button, and completion keybinding from defaults, persistence, migration, and Settings.

## 3. Diagnostic markers

- [x] 3.1 Map current Tinymist error and warning diagnostics with source ranges to severity-colored curvy source-view underlines.
- [x] 3.2 Show the diagnostic message when the user hovers its underline, clear markers immediately after source changes, and apply only diagnostics for the current document version.
- [x] 3.3 Keep the last successful preview visible when current Tinymist diagnostics contain errors.

## 4. Project navigation and exit safety

- [x] 4.1 Put the project file list in a vertical scroller without adding horizontal scrolling.
- [x] 4.2 Start each opened project with every folder collapsed while preserving normal expand/collapse behavior during the session.
- [ ] 4.3 Before application exit with dirty source, show Save, Discard, and Cancel; exit only after a successful save or explicit discard, and remain open after cancellation or save failure.
- [ ] 4.4 Clear the autosave after successful Save or Discard so a clean exit does not show recovery on the next run; retain autosave recovery for crashes or interrupted sessions.

## 5. Menu organization and About

- [x] 5.1 Move Capture and Settings into Edit, move Export PDF into File, keep preview controls in View, and remove the separate Capture menu.
- [x] 5.2 Add About as its own top-level menu button opening the application About dialog with name, version, GPL-3.0-or-later license, repository link, and bundled Typst/Tinymist acknowledgements.

## 6. Licensing

- [x] 6.1 Change workspace package metadata to GPL-3.0-or-later to match the repository LICENSE.
- [x] 6.2 Document bundled Tinymist beside Typst with its version, source URL, checksum, Apache-2.0 license, and retained upstream notices.

## 7. Validation

- [ ] 7.1 Add focused protocol tests for initialization, document synchronization, completion edits, diagnostics, stale responses, and unavailable or terminated Tinymist.
- [ ] 7.2 Add focused UI/state tests for popup keyboard and pointer behavior, diagnostic marker refresh, scrollable collapsed trees, menu placement, About metadata, and every dirty-exit choice including save failure.
- [ ] 7.3 Run formatting, Clippy, workspace tests, OpenSpec validation, dependency/license checks, and the AppImage packaging check.
- [ ] 7.4 Manually verify keyboard-only completion, accessible diagnostic feedback, large project-tree scrolling, command placement, About dialog, and clean restart after Save and Discard exits.
