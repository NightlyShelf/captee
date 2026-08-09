# workspace-management Specification

## Purpose

Provides a safe, portable project boundary for Typst sources, image assets, preferences, and recent-project metadata.
## Requirements
### Requirement: Project lifecycle

The application SHALL create and open a Captee project rooted at a user-selected directory, and SHALL identify the entry Typst document and project configuration from that root.

#### Scenario: Create a project

- **WHEN** a user selects an empty directory and supplies a project name
- **THEN** the application creates the project configuration, entry Typst file, and `img/` asset directory
- **AND** the created project becomes the active project

#### Scenario: Open an existing project

- **WHEN** a user selects a directory containing a valid Captee configuration
- **THEN** the application loads its entry document and project settings without modifying unrelated files

#### Scenario: Reject an invalid project

- **WHEN** a selected directory is missing required configuration or contains an invalid configuration
- **THEN** the application reports a recoverable validation error and leaves the directory unchanged

### Requirement: Safe file operations

The application SHALL restrict project-managed writes to the active project root, preserve unrelated files, and use atomic replacement for configuration and autosave writes.

#### Scenario: Prevent path escape

- **WHEN** a requested project-relative path resolves outside the project root
- **THEN** the operation fails with a validation error and performs no filesystem mutation

#### Scenario: Recover from interrupted autosave

- **WHEN** an autosave is interrupted during a write
- **THEN** the previous complete file remains readable and a later startup can recover the newest complete autosave

### Requirement: Recent projects and trash

The application SHALL maintain a bounded, deduplicated recent-project list and SHALL require explicit confirmation before moving a project item to the system trash.

#### Scenario: Update recent projects

- **WHEN** a project is opened successfully
- **THEN** it is moved to the front of the recent list, duplicate entries are removed, and the list is persisted

#### Scenario: Cancel trash operation

- **WHEN** a user declines a trash confirmation
- **THEN** the selected item remains in place and no project state changes

### Requirement: Safe project tree mutations

Project tree create, move, and delete actions SHALL operate only on validated project-relative paths, preserve the active project boundary, and leave the project unchanged when validation fails or confirmation is declined. Moving a folder into itself or one of its descendants SHALL be rejected.

#### Scenario: Create a project file or folder

- **WHEN** the user confirms creation of a valid file or folder name under a project folder
- **THEN** the item is created inside the active project and the project tree can display it

#### Scenario: Move a project item

- **WHEN** the user confirms moving an item to a valid destination folder in the active project
- **THEN** the item and its descendants are moved atomically where supported, and no path outside the project root is accessed

#### Scenario: Rename a project item

- **WHEN** the user triple-clicks a tree item or chooses Rename, enters a safe new name, and confirms
- **THEN** the item is renamed in its existing parent and the tree reflects the new name

#### Scenario: Move an item to the project root

- **WHEN** the user chooses the project root as the destination
- **THEN** the item is moved to the root without escaping the active project boundary

#### Scenario: Reject an unsafe move

- **WHEN** the requested move escapes the project root, targets the item itself, or targets a descendant of a moved folder
- **THEN** the operation is rejected with an actionable message and no filesystem mutation occurs

#### Scenario: Confirmed deletion

- **WHEN** the user confirms deletion of a project item
- **THEN** the existing safe trash boundary is used and the project tree reflects the result only after the operation succeeds

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
