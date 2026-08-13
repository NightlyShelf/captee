## ADDED Requirements

### Requirement: Editor assistance

The application SHALL automatically provide context-aware Typst completion in a cursor-anchored popup when the user types `#`, and visually mark current compiler diagnostics with curvy underlines at their source spans. Hovering a diagnostic underline SHALL reveal its details. Completion and diagnostics SHALL not block normal source editing. The application SHALL not expose a manual completion command, button, or keybinding.

#### Scenario: Request completion

- **WHEN** the user types `#` in a Typst editor
- **THEN** matching candidates appear in a popup anchored near the cursor
- **AND** the user can accept a candidate with keyboard or pointer input

#### Scenario: Dismiss completion

- **WHEN** the completion popup is open and the user presses Escape or continues with unrelated input
- **THEN** the popup closes without changing the source

#### Scenario: Mark current compiler error

- **WHEN** the latest render reports a compiler diagnostic with a source span
- **THEN** the corresponding source range is marked with a curvy underline
- **AND** hovering it exposes its diagnostic message

#### Scenario: Clear stale markers

- **WHEN** source changes or a newer render replaces diagnostics
- **THEN** markers from the older source revision are removed
