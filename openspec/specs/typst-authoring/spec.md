# typst-authoring Specification

## Purpose

Provides responsive Typst source editing with useful feedback while keeping compiler and editor concerns testable independently of the desktop UI.
## Requirements
### Requirement: Source editing and persistence

The application SHALL display and edit the active Typst source, track unsaved changes, and save through the workspace's atomic write boundary.

#### Scenario: Edit source

- **WHEN** a user changes text in the active document
- **THEN** the editor reflects the change immediately and marks the document dirty

#### Scenario: Save source

- **WHEN** a user saves a dirty document
- **THEN** the complete source is atomically written to the active project's entry file and the dirty state is cleared

### Requirement: Diagnostics and formatting

The application SHALL run Typst compilation and formatting asynchronously, expose structured diagnostics with source locations, and discard results belonging to superseded source revisions.

#### Scenario: Show compiler diagnostics

- **WHEN** compilation reports an error or warning
- **THEN** the editor displays its severity, message, and source span and keeps the source editable

#### Scenario: Discard stale compilation

- **WHEN** a newer edit is submitted before an earlier compilation completes
- **THEN** the earlier result is not applied to the current document

#### Scenario: Format source

- **WHEN** the user invokes formatting and the formatter succeeds
- **THEN** the editor replaces the source with formatted text and marks it dirty
- **AND** when formatting fails, the original source remains unchanged and the error is shown

### Requirement: Completion and search

The application SHALL provide context-aware completion candidates and literal find/replace operations without changing text until the user confirms a replacement.

#### Scenario: Search and replace

- **WHEN** a user searches for a literal term and confirms replacement
- **THEN** only matching occurrences in the selected scope are replaced and the edit becomes undoable

#### Scenario: Completion cancellation

- **WHEN** a user dismisses completion without selecting a candidate
- **THEN** the source remains unchanged

### Requirement: Typst editor language presentation

Every Typst text editor in the workspace, including the main source editor and the capture annotation editor, SHALL provide Typst syntax highlighting for the same language constructs and SHALL preserve ordinary text editing, selection, undo, and diagnostics behavior.

#### Scenario: Highlight main editor

- **WHEN** the user opens a Typst source file in the main editor
- **THEN** Typst markup, commands, strings, comments, numbers, delimiters, and diagnostics are presented with syntax-aware styling without changing the source text

#### Scenario: Highlight capture editor

- **WHEN** the user edits annotation code in the capture review
- **THEN** the same Typst syntax-aware styling is applied in the annotation editor

#### Scenario: Show an empty annotation editor

- **WHEN** the capture review opens
- **THEN** the annotation buffer is empty and a dimmed Typst description placeholder is visible until the user types

#### Scenario: Use the dark editor presentation

- **WHEN** the workspace uses a dark desktop theme
- **THEN** the editor surface, line-number gutter, syntax text, and caret retain visible contrasting colors without a white gutter or dark-on-dark caret

### Requirement: Typst command autocomplete in every editor

Every Typst text editor in the workspace SHALL offer command autocomplete, including the main source editor and capture annotation editor. Suggestions SHALL be derived from the current Typst command context and SHALL be dismissible without changing text.

#### Scenario: Complete a Typst command

- **WHEN** the user invokes autocomplete or types a command prefix in either Typst editor
- **THEN** the editor presents matching Typst command suggestions and accepts the selected command at the current cursor position

#### Scenario: Dismiss command autocomplete

- **WHEN** the user dismisses the autocomplete list or no suggestion is selected
- **THEN** neither editor changes its source text
