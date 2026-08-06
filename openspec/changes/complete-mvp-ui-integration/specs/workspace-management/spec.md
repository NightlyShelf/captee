## ADDED Requirements

### Requirement: Connected project persistence

The workspace MUST connect project state to entry-document persistence, autosave/recovery, recent-project tracking, and safe project actions without allowing paths outside the active project boundary.

#### Scenario: Autosave and recovery protect edits

- **WHEN** an edited document is autosaved or the application restarts after an interrupted write
- **THEN** the latest complete autosave can be recovered without corrupting the main entry document

#### Scenario: Recent project is recorded

- **WHEN** a project opens successfully
- **THEN** its path is persisted at the front of a bounded, deduplicated recent-project list

#### Scenario: Cancelled destructive action is harmless

- **WHEN** the user declines a project-item trash confirmation
- **THEN** the item remains in place and no project state changes

## ADDED Requirements

### Requirement: Safe project tree mutations

Project tree create, move, and delete actions SHALL operate only on validated project-relative paths, preserve the active project boundary, and leave the project unchanged when validation fails or confirmation is declined. Moving a folder into itself or one of its descendants SHALL be rejected.

#### Scenario: Create a project file or folder

- **WHEN** the user confirms creation of a valid file or folder name under a project folder
- **THEN** the item is created inside the active project and the project tree can display it

#### Scenario: Move a project item

- **WHEN** the user confirms moving an item to a valid destination folder in the active project
- **THEN** the item and its descendants are moved atomically where supported, and no path outside the project root is accessed

#### Scenario: Reject an unsafe move

- **WHEN** the requested move escapes the project root, targets the item itself, or targets a descendant of a moved folder
- **THEN** the operation is rejected with an actionable message and no filesystem mutation occurs

#### Scenario: Confirmed deletion

- **WHEN** the user confirms deletion of a project item
- **THEN** the existing safe trash boundary is used and the project tree reflects the result only after the operation succeeds
