## ADDED Requirements

### Requirement: Connected source authoring

The active GTK source editor MUST update the core document state, preserve revision and dirty semantics, and route save, formatting, search/replace, and completion through testable boundaries.

#### Scenario: Edit and save source

- **WHEN** the user edits the active source and invokes Save
- **THEN** the active entry document is atomically written and the dirty indicator clears only after persistence succeeds

#### Scenario: Formatting failure preserves source

- **WHEN** formatting fails
- **THEN** the editor source remains unchanged and the error is shown

#### Scenario: Confirmed replacement is undoable

- **WHEN** the user confirms a literal find/replace operation
- **THEN** only the selected matches change and the edit can be undone

#### Scenario: Completion cancellation is harmless

- **WHEN** the user dismisses completion or a stale completion result arrives
- **THEN** the source remains unchanged

### Requirement: Connected authoring diagnostics

Compiler and formatter diagnostics MUST be rendered with severity and source locations when available, while leaving the source editable.

#### Scenario: Diagnostic result is shown

- **WHEN** an authoring operation returns diagnostics
- **THEN** the editor or diagnostics surface displays the message and location without replacing the source
