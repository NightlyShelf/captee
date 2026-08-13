## MODIFIED Requirements

### Requirement: Completion and search

The application SHALL use the bundled Tinymist language server for context-aware completion in every Typst editor and SHALL provide literal find/replace operations without changing text until the user confirms a replacement. Completion requests and responses SHALL be tied to the current document version.

#### Scenario: Search and replace

- **WHEN** a user searches for a literal term and confirms replacement
- **THEN** only matching occurrences in the selected scope are replaced and the edit becomes undoable

#### Scenario: Request completion

- **WHEN** a user types a Typst command prefix in either Typst editor
- **THEN** Tinymist returns completion candidates for that document and cursor position

#### Scenario: Completion cancellation

- **WHEN** a user dismisses completion without selecting a candidate
- **THEN** the source remains unchanged

#### Scenario: Discard stale completion

- **WHEN** the document changes before a completion response arrives
- **THEN** the stale response does not change the source or current popup
