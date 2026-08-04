## MODIFIED Requirements

### Requirement: Project home and workspace

The application SHALL provide a project home for creating/opening projects and a three-pane workspace containing navigation, source editing, and preview areas. Creating a project SHALL collect its name and parent location before creation, while opening a project SHALL allow selecting its folder.

#### Scenario: New project dialog

- **WHEN** the user activates New Project
- **THEN** the UI presents a dialog with a project-name field, a location chooser, and an explicit create action

#### Scenario: Open project dialog

- **WHEN** the user activates Open Project
- **THEN** the UI presents a folder-selection dialog for choosing an existing project

#### Scenario: Open workspace

- **WHEN** a project is opened or created successfully
- **THEN** the workspace shows the project files, active Typst source, preview state, and active project context

#### Scenario: Empty home

- **WHEN** no project is active
- **THEN** the home view offers create and open actions without exposing project-only menu controls

#### Scenario: Workspace menu placement

- **WHEN** a project is active
- **THEN** compact regular menu buttons are shown in a left-aligned workspace menu strip and are not duplicated in the home view or global header

#### Scenario: Close workspace

- **WHEN** the user activates Close Project with no blocking operation running
- **THEN** the application returns to the home view, clears the editor contents, and removes the active project context
