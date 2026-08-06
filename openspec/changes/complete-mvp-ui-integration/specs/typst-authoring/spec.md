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

## ADDED Requirements

### Requirement: Typst editor language presentation

Every Typst text editor in the workspace, including the main source editor and the capture annotation editor, SHALL provide Typst syntax highlighting for the same language constructs and SHALL preserve ordinary text editing, selection, undo, and diagnostics behavior.

#### Scenario: Highlight main editor

- **WHEN** the user opens a Typst source file in the main editor
- **THEN** Typst markup, commands, strings, comments, numbers, delimiters, and diagnostics are presented with syntax-aware styling without changing the source text

#### Scenario: Highlight capture editor

- **WHEN** the user edits annotation code in the capture review
- **THEN** the same Typst syntax-aware styling is applied in the annotation editor

### Requirement: Typst command autocomplete in every editor

Every Typst text editor in the workspace SHALL offer command autocomplete, including the main source editor and capture annotation editor. Suggestions SHALL be derived from the current Typst command context and SHALL be dismissible without changing text.

#### Scenario: Complete a Typst command

- **WHEN** the user invokes autocomplete or types a command prefix in either Typst editor
- **THEN** the editor presents matching Typst command suggestions and accepts the selected command at the current cursor position

#### Scenario: Dismiss command autocomplete

- **WHEN** the user dismisses the autocomplete list or no suggestion is selected
- **THEN** neither editor changes its source text
