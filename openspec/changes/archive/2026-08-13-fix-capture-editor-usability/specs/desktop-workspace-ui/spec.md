## MODIFIED Requirements

### Requirement: Commands and settings

The application SHALL expose menu and keyboard commands for save, capture, format, find/replace, preview, and export. Formatting, capture fallback, and preview preferences SHALL persist per project. Keybindings SHALL persist as user-global settings outside project configuration and apply consistently whenever a project is open.

#### Scenario: Change a setting

- **WHEN** a user changes a setting and confirms it
- **THEN** the setting is validated, persisted, and applied to subsequent operations

#### Scenario: Change a keybinding

- **WHEN** a user changes a keybinding and confirms it
- **THEN** the user-global keybinding is persisted and applied without changing project configuration

#### Scenario: Invalid setting

- **WHEN** a setting value fails validation
- **THEN** the UI explains the constraint, retains the prior value, and does not persist the invalid value

## ADDED Requirements

### Requirement: Global capture shortcut

The application SHALL register the configured user-global capture shortcut only while a project is open. The default capture shortcut SHALL be Ctrl+~. Changing its keybinding SHALL replace the active system registration.

#### Scenario: Update global capture shortcut

- **WHEN** a user saves a changed capture keybinding while a project is open
- **THEN** the prior system shortcut is released and the new shortcut starts capture

#### Scenario: Close project

- **WHEN** the user closes the active project
- **THEN** the system capture shortcut is released

### Requirement: Capture editor navigation

The capture review editor SHALL focus and reveal the editable annotation at its insertion point, positioned a few lines above the bottom edge with sufficient trailing editor space, without horizontal scrolling. After confirmed capture insertion, the source editor SHALL focus and reveal the inserted image and annotation, while the preview scrolls to its end.

#### Scenario: Open capture review

- **WHEN** capture review opens with preceding source context
- **THEN** the editable annotation insertion point is visible and focused a few lines above the editor bottom
- **AND** the review editor has no horizontal scrollbar

#### Scenario: Reach final source line

- **WHEN** the source editor cursor reaches the final document line
- **THEN** bottom editor space allows that line to sit above the editor bottom edge

#### Scenario: Confirm capture insertion

- **WHEN** a confirmed capture is inserted into the source
- **THEN** the source editor focuses and reveals the inserted content
- **AND** the preview scrolls to its end
