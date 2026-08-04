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
