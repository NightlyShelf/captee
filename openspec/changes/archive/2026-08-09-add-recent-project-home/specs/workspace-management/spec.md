## ADDED Requirements

### Requirement: Recent-project preferences and removal

The application SHALL persist each recent project's path, name, last-access time, and pin state. It SHALL provide a bounded displayed list of five projects, ordered by pinned state then last access. Removing a project from the list SHALL preserve its directory; deleting from disk SHALL use the existing safe system-trash boundary and remove the entry only after that operation succeeds.

#### Scenario: Update a project access time

- **WHEN** a project opens successfully
- **THEN** its last-access time is updated and the displayed list is reordered

#### Scenario: Remove a project from the list

- **WHEN** the user confirms Remove from list
- **THEN** the project entry is removed from saved recent projects
- **AND** its directory is unchanged

#### Scenario: Delete a project from disk

- **WHEN** the user confirms Delete from disk
- **THEN** the project directory is moved through the system-trash boundary
- **AND** its saved recent-project entry is removed after success

#### Scenario: Cancel recent-project deletion

- **WHEN** the user cancels the deletion dialog
- **THEN** the saved entry and project directory remain unchanged
