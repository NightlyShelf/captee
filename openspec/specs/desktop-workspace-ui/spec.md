# desktop-workspace-ui Specification

## Purpose

Presents a discoverable Linux desktop workflow for project selection, Typst authoring, capture, preview, export, and settings with accessible interaction states.
## Requirements
### Requirement: Project home and workspace

The application SHALL provide a project home for creating/opening projects and a workspace containing an interactive project tree, source editing area, and preview area. Creating a project SHALL collect its name and parent location before creation, while opening a project SHALL allow selecting its folder. When the workspace first opens, the project tree SHALL occupy approximately one sixth of the window width and the remaining area SHALL be divided approximately equally between the editor and preview. The project tree SHALL show the project name, file/folder add actions, and recursively nested project files and folders; file and folder rows SHALL be clickable, and valid drag-and-drop moves SHALL be supported.

#### Scenario: New project dialog

- **WHEN** the user activates New Project
- **THEN** the UI presents a dialog with a project-name field, a location chooser, and an explicit create action

#### Scenario: Open project dialog

- **WHEN** the user activates Open Project
- **THEN** the UI presents a folder-selection dialog for choosing an existing project

#### Scenario: Open workspace

- **WHEN** a project is opened or created successfully
- **THEN** the workspace shows the project files, active Typst source, preview state, and active project context

#### Scenario: Open workspace layout

- **WHEN** a project is opened or created successfully
- **THEN** the workspace shows the project tree, active Typst source, preview state, and active project context, with the tree initially using about one sixth of the width and the remaining space split 50/50 between editor and preview

#### Scenario: Navigate the project tree

- **WHEN** the user clicks a file or folder row
- **THEN** a file opens in the appropriate editor or a folder expands/collapses without losing the active project context

#### Scenario: Drag a project item

- **WHEN** the user drags a file or folder onto a valid destination folder within the active project
- **THEN** the item is moved through the safe project operation boundary and the tree refreshes to show the new hierarchy

#### Scenario: Show a stable file hierarchy

- **WHEN** the project tree is rendered
- **THEN** entries are shown in recursive parent-before-child order with visible indentation, folder expand/collapse state, and an icon appropriate to each file or folder type

#### Scenario: Move an item to the project root

- **WHEN** the user drags an item onto the project tree root or chooses an empty destination in the Move action
- **THEN** the item is moved to the project root and the refreshed hierarchy shows it at the top level

#### Scenario: Rename an item by triple click

- **WHEN** the user triple-clicks a file or folder row, enters a safe single-component name, and confirms
- **THEN** an inline text field replaces the row label at that file or folder position, the item is renamed in place, and the tree refreshes without changing its parent

#### Scenario: Resize and truncate the project panel

- **WHEN** the project panel is narrower than a file name or the user drags its divider
- **THEN** the panel can be resized, long names ellipsize instead of changing the editor/preview structure, and the editor and preview retain an equal initial split of the remaining width

#### Scenario: Workspace menu placement

- **WHEN** a project is active
- **THEN** compact, squared, regular menu items such as File and Edit are shown in a left-aligned menu strip with minimal spacing, no rounded button treatment, and no visible button stroke, while a horizontal separator divides the menu from the workspace

#### Scenario: Global capture shortcut

- **WHEN** the configured global capture shortcut is pressed while Captee is not focused
- **THEN** the capture selection starts without requiring a workspace focus change

#### Scenario: Project tree toolbar actions

- **WHEN** a project is active
- **THEN** the project name and small add-file and add-folder icon actions are visible at the top of the project tree

#### Scenario: Empty home

- **WHEN** no project is active
- **THEN** the home view offers create and open actions without exposing project-only menu controls

#### Scenario: Close workspace

- **WHEN** the user activates Close Project with no blocking operation running
- **THEN** the application returns to the home view, clears the editor contents, and removes the active project context

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

### Requirement: Accessible interaction states

The UI SHALL expose keyboard focus, labels, progress, success, warning, and error states to assistive technologies and SHALL avoid blocking the UI thread during compilation, rendering, or capture.

#### Scenario: Long-running operation

- **WHEN** compilation, rendering, or capture is in progress
- **THEN** the UI remains responsive, exposes progress or cancellation where supported, and prevents conflicting destructive actions

#### Scenario: Operation failure

- **WHEN** an operation fails
- **THEN** the UI presents an actionable error and leaves the last valid project state intact

### Requirement: Project tree context actions

The project tree SHALL provide a context menu for creating files or folders, moving items, and deleting items. Create, move, and delete actions SHALL show a small confirmation dialog before mutation; declining or cancelling the dialog SHALL leave the project and tree unchanged.

#### Scenario: Create an item from the context menu

- **WHEN** the user chooses New File or New Folder, supplies a valid project-relative name, and confirms the small dialog
- **THEN** the item is created under the selected folder and appears in the tree

#### Scenario: Move an item from the context menu

- **WHEN** the user chooses Move, selects a valid destination, and confirms the small dialog
- **THEN** the item is moved and the tree reflects the updated hierarchy

#### Scenario: Delete an item from the context menu

- **WHEN** the user chooses Delete and confirms the small dialog
- **THEN** the item is moved to the system trash or removed through the existing safe deletion boundary and disappears from the tree

#### Scenario: Decline a context action

- **WHEN** the user cancels or declines a create, move, or delete confirmation
- **THEN** no filesystem or source mutation occurs and the current selection remains usable

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
