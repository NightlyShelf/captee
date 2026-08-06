## ADDED Requirements

### Requirement: Workspace actions execute end to end

Workspace menu and keyboard actions MUST invoke their corresponding core/platform operation, report progress or cancellation where supported, and return to an idle state with a success or actionable failure announcement.

#### Scenario: Capture action completes or fails visibly

- **WHEN** the user invokes Capture from the workspace
- **THEN** the UI runs the capture workflow and presents completion, cancellation, or failure instead of remaining indefinitely busy

#### Scenario: Operation cancellation restores idle state

- **WHEN** the user cancels a cancellable operation
- **THEN** the UI stops or disregards the operation result, clears progress, and leaves the last valid project state unchanged

#### Scenario: Action failure preserves project state

- **WHEN** a workspace operation fails
- **THEN** the UI reports the failure and keeps the active source, project, and last valid preview intact

### Requirement: Accessible operation feedback

The workspace MUST expose operation status, errors, and cancellation through visible and accessible controls without blocking the GTK main loop during platform work.

#### Scenario: Long-running operation remains responsive

- **WHEN** compilation, capture, or filesystem work is running
- **THEN** the workspace remains interactive for supported cancellation and exposes the current operation state

## MODIFIED Requirements

### Requirement: Project home and workspace

The application SHALL provide a project home for creating/opening projects and a workspace containing an interactive project tree, source editing area, and preview area. Creating a project SHALL collect its name and parent location before creation, while opening a project SHALL allow selecting its folder. When the workspace first opens, the project tree SHALL occupy approximately one sixth of the window width and the remaining area SHALL be divided approximately equally between the editor and preview. The project tree SHALL show the project name, file/folder add actions, and recursively nested project files and folders; file and folder rows SHALL be clickable, and valid drag-and-drop moves SHALL be supported.

#### Scenario: New project dialog

- **WHEN** the user activates New Project
- **THEN** the UI presents a dialog with a project-name field, a location chooser, and an explicit create action

#### Scenario: Open project dialog

- **WHEN** the user activates Open Project
- **THEN** the UI presents a folder-selection dialog for choosing an existing project

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

## ADDED Requirements

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
