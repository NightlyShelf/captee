## ADDED Requirements

### Requirement: Recent-project home panel

The project home SHALL show a centered, non-scrollable Latest projects panel using existing Captee colors and UI style. It SHALL provide New Project and Open Existing actions and show at most five projects: pinned projects first, then most-recent non-pinned projects. Each project row SHALL show its name, path, last-access date, pin icon action, and delete icon action; selecting its name SHALL open that project.

#### Scenario: Show recent projects

- **WHEN** the project home opens with saved projects
- **THEN** it shows at most five rows with pinned projects before recent projects
- **AND** the panel does not scroll

#### Scenario: Open a listed project

- **WHEN** the user selects a project name
- **THEN** the project opens through the existing project-open flow

#### Scenario: Pin a project

- **WHEN** the user activates a project's pin icon
- **THEN** the home panel updates to show that project before non-pinned projects

#### Scenario: Confirm recent-project deletion

- **WHEN** the user activates a project's delete icon
- **THEN** a dialog offers Remove from list, Delete from disk, and Cancel
- **AND** cancelling leaves the list and project directory unchanged
