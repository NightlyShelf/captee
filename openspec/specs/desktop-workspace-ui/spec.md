# desktop-workspace-ui Specification

## Purpose

Presents a discoverable Linux desktop workflow for project selection, Typst authoring, capture, preview, export, and settings with accessible interaction states.

## Requirements

### Requirement: Project home and workspace

The application SHALL provide a project home for creating/opening projects and a three-pane workspace containing navigation, source editing, and preview areas.

#### Scenario: Open workspace

- **WHEN** a project is opened successfully
- **THEN** the workspace shows the project files, active Typst source, and preview state

#### Scenario: Empty home

- **WHEN** no project is active
- **THEN** the home view offers create, open, and recent-project actions without exposing project-only controls

### Requirement: Commands and settings

The application SHALL expose menu and keyboard commands for save, capture, format, find/replace, preview, and export, and SHALL persist settings for keybindings, formatting, capture fallback, and preview preferences.

#### Scenario: Change a setting

- **WHEN** a user changes a setting and confirms it
- **THEN** the setting is validated, persisted, and applied to subsequent operations

#### Scenario: Invalid setting

- **WHEN** a setting value fails validation
- **THEN** the UI explains the constraint, retains the prior value, and does not persist the invalid value

### Requirement: Accessible interaction states

The UI SHALL expose keyboard focus, labels, progress, success, warning, and error states to assistive technologies and SHALL avoid blocking the UI thread during compilation, rendering, or capture.

#### Scenario: Long-running operation

- **WHEN** compilation, rendering, or capture is in progress
- **THEN** the UI remains responsive, exposes progress or cancellation where supported, and prevents conflicting destructive actions

#### Scenario: Operation failure

- **WHEN** an operation fails
- **THEN** the UI presents an actionable error and leaves the last valid project state intact
