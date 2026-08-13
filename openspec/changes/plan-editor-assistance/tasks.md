## 1. Completion popup

- [ ] 1.1 Reuse Typst completion candidates for the main source editor and capture-review editor.
- [ ] 1.2 Automatically show a cursor-anchored popup when the user types `#`, with keyboard and pointer selection, acceptance, and dismissal.
- [ ] 1.3 Remove the manual completion command, button, and keybinding.
- [ ] 1.4 Keep completion edits limited to the replacement range and preserve normal editor input when no popup is active.

## 2. Diagnostic markers

- [ ] 2.1 Map current Typst compiler diagnostics with source spans to curvy source-view underlines.
- [ ] 2.2 Show diagnostic details when the user hovers an underline, and clear markers when their source revision is replaced or diagnostics become stale.
- [ ] 2.3 Keep diagnostics visible after a failed render while retaining the last successful preview.

## 3. Validation

- [ ] 3.1 Add focused tests for popup candidate selection, replacement ranges, dismissal, and diagnostic span/refresh behavior.
- [ ] 3.2 Manually verify keyboard-only completion and accessible diagnostic feedback.
