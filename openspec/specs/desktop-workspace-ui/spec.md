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

### Requirement: Tinymist editor assistance

The application SHALL use the bundled Tinymist language server to provide context-aware completion and current diagnostics in the main source editor and capture-review editor. Tinymist SHALL run locally for the active project and SHALL not replace the existing Typst compiler used for formatting, preview, and export. Completion and diagnostics SHALL not block normal editing or other authoring operations when Tinymist is unavailable. The application SHALL not expose a manual completion command, button, or keybinding.

#### Scenario: Request completion

- **WHEN** the user types `#` in a Typst editor
- **THEN** Tinymist matching candidates appear in a popup anchored near the cursor and update while the command prefix changes
- **AND** the user can select with Up/Down and accept with Enter, Tab, or pointer input

#### Scenario: Dismiss completion

- **WHEN** the completion popup is open and the user presses Escape or types unrelated input
- **THEN** the popup closes without changing the source

#### Scenario: Apply completion edit

- **WHEN** the user accepts a current Tinymist completion
- **THEN** only its returned text edit or replacement range changes
- **AND** a stale or empty response does not change the source

#### Scenario: Mark current compiler error

- **WHEN** Tinymist reports a current error or warning with a source range
- **THEN** the corresponding source range is marked with a severity-colored curvy underline
- **AND** hovering it exposes its diagnostic message

#### Scenario: Clear stale markers

- **WHEN** source changes or a newer render replaces diagnostics
- **THEN** markers from the older source revision are removed

#### Scenario: Summarize current errors

- **WHEN** current Tinymist diagnostics are displayed in the main editor
- **THEN** a tiny transparent indicator at the editor's bottom-right shows a centered white cross on red with the error count
- **AND** when there are no current errors it shows a centered white tick on green with “No errors”

#### Scenario: Restore the reopened workspace view

- **WHEN** the user closes and later reopens a project
- **THEN** the editor restores the saved caret and editor viewport from project-local hidden view state
- **AND** keyboard focus returns to the restored source caret
- **AND** the preview restores its saved viewport after the first rendered page layout is ready
- **AND** ordinary edits and renders do not force either pane to a different position

#### Scenario: Reuse project capture and preview preferences

- **WHEN** the user changes capture insertion between before and after the image or toggles preview auto-scroll
- **THEN** the selected behavior takes effect immediately and is saved in project-local view state
- **AND** the next capture and the next project reopen reuse those choices

#### Scenario: Modify an annotated capture

- **WHEN** the user modifies a capture after typing annotation text
- **THEN** that text remains editable without the annotation placeholder overlapping it

#### Scenario: Tinymist unavailable

- **WHEN** Tinymist cannot start or exits unexpectedly
- **THEN** the application reports that editor assistance is unavailable
- **AND** editing, saving, formatting, preview, and export remain usable

### Requirement: Scrollable collapsed project tree

The project file list SHALL scroll vertically within the project panel without horizontal scrolling. Every newly opened project SHALL initially show all folders collapsed, while allowing folders to be expanded and collapsed for the rest of that project session.

#### Scenario: Open project tree

- **WHEN** a project with nested folders opens
- **THEN** every folder is collapsed and root entries remain visible

#### Scenario: Browse a large project tree

- **WHEN** visible project entries exceed the panel height
- **THEN** the user can scroll vertically to every visible entry
- **AND** long names remain truncated without horizontal scrolling

### Requirement: Confirm dirty application exit

The application SHALL prompt before exiting when the active source has unsaved changes. The prompt SHALL offer Save, Discard, and Cancel, and clean Save or Discard exits SHALL remove the autosave so the next normal run does not offer recovery. Autosave SHALL remain available for recovery after a crash or interrupted session.

#### Scenario: Save before exit

- **WHEN** the user chooses Save in the dirty-exit prompt
- **THEN** the source is saved atomically, the autosave is cleared, and the application exits
- **AND** a save failure is shown while the application remains open

#### Scenario: Discard before exit

- **WHEN** the user chooses Discard in the dirty-exit prompt
- **THEN** the autosave is cleared without writing dirty source to the project and the application exits

#### Scenario: Cancel exit

- **WHEN** the user chooses Cancel or closes the dirty-exit prompt
- **THEN** the application remains open with source and autosave unchanged

#### Scenario: Exit clean source

- **WHEN** the user exits with no unsaved source changes
- **THEN** the application exits without showing the dirty-exit prompt

### Requirement: Workspace menu organization

The workspace menu strip SHALL group project and output commands under File, editing and configuration commands under Edit, and preview presentation commands under View. Capture and Settings SHALL appear under Edit, Export PDF SHALL appear under File, and no separate Capture menu SHALL remain. About SHALL be a separate top-level menu button that opens application identity, version, license, repository, and bundled-tool acknowledgements.

#### Scenario: Show workspace command groups

- **WHEN** a project workspace is active
- **THEN** File contains Export PDF, Edit contains Capture and Settings, and View contains preview controls
- **AND** no separate Capture menu is shown

#### Scenario: Open About

- **WHEN** the user activates the separate About menu button
- **THEN** an About dialog shows Captee's name, version, GPL-3.0-or-later license, repository link, and Typst and Tinymist acknowledgements
