## ADDED Requirements

### Requirement: Connected preview rendering

The workspace MUST invoke asynchronous Typst preview compilation from the active source and apply results only when they belong to the current source revision.

#### Scenario: Current preview is displayed

- **WHEN** compilation succeeds for the current source revision
- **THEN** the preview pane displays the rendered document and render status

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
