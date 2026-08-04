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
