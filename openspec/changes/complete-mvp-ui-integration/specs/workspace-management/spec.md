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
