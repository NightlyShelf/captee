## Why

Project actions currently provide no reliable visible transition when users open or close a project, and the home view repeats workspace-only controls. This makes the primary workflow ambiguous and prevents users from confidently knowing which surface is active.

## What Changes

- Make New Project open a dialog that collects the project name and parent location before creating the project.
- Make Open Project open a folder-selection dialog and load the selected project.
- Make successful project creation/opening switch to the workspace and update the active project context reliably.
- Make closing a project return to the home surface and clear the editor context.
- Keep project-only menus out of the home surface.
- Present compact workspace menu buttons aligned to the left instead of duplicating project actions in the header and home controls.
- Add regression coverage for the lifecycle transitions and menu-surface rules.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `desktop-workspace-ui`: project home/workspace transitions and project-only menu placement are clarified and made observable.

## Impact

- `crates/captee-ui/src/native.rs` GTK layout and action wiring.
- `crates/captee-ui` headless UI-state tests where applicable.
- `openspec/specs/desktop-workspace-ui/spec.md` delta requirements and the active change's performance/architecture records.
