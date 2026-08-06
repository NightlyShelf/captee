## ADDED Requirements

### Requirement: Connected preview rendering

The workspace MUST invoke asynchronous Typst preview compilation from the active source and apply results only when they belong to the current source revision. While preview compilation is running, the source editor SHALL remain editable; editing the source SHALL cancel the superseded preview and leave the workspace usable for the next debounced render.

#### Scenario: Current preview is displayed

- **WHEN** compilation succeeds for the current source revision
- **THEN** the preview pane displays every rendered page in document order and render status

#### Scenario: Edit during rendering

- **WHEN** the user edits the source while a preview compilation is running
- **THEN** the editor accepts the edit, the superseded render is discarded, and a new debounced preview may be started

#### Scenario: Stale preview is ignored

- **WHEN** a newer source revision exists before an older compile completes
- **THEN** the older result cannot replace the current preview

### Requirement: Connected PDF export

The workspace MUST offer a destination dialog and export only a successful preview matching the current source revision using atomic replacement.

#### Scenario: Export current preview

- **WHEN** the user exports with a current successful preview
- **THEN** a complete PDF is written to the selected destination

#### Scenario: Export without current preview

- **WHEN** no current successful preview exists
- **THEN** export is refused with an actionable message and the destination is unchanged
