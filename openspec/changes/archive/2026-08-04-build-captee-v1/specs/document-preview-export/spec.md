## Purpose

Turns the selected Typst entry document into a current rendered preview and a portable PDF while reporting failures without losing source edits.

## ADDED Requirements

### Requirement: Asynchronous preview
The application SHALL render the selected entry document asynchronously and SHALL associate each preview with the source revision that produced it.

#### Scenario: Update preview
- **WHEN** a render completes for the current source revision
- **THEN** the preview pane displays the resulting document and its render timestamp

#### Scenario: Ignore stale preview
- **WHEN** a newer source revision exists before an older render completes
- **THEN** the older render is discarded and cannot replace the newer preview

### Requirement: Render diagnostics
The application SHALL expose render errors and warnings with source locations when the compiler provides them, while retaining the last successful preview.

#### Scenario: Failed render
- **WHEN** rendering fails
- **THEN** the preview pane shows the diagnostics and keeps the last successful rendered output available

### Requirement: PDF export
The application SHALL export the current successful render to a user-selected PDF destination using an atomic file replacement.

#### Scenario: Export successful render
- **WHEN** a user exports and a successful render exists for the current source revision
- **THEN** a complete PDF is written to the selected destination

#### Scenario: Export without current render
- **WHEN** no successful render exists for the current source revision
- **THEN** export is refused with an actionable message and no destination file is changed
