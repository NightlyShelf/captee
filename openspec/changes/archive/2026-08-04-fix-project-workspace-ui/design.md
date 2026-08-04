## Context

The native GTK adapter currently combines home actions, workspace actions, and a File popover in one global header. Project creation derives the name from a selected directory, and the project lifecycle updates widgets at several call sites. See `proposal.md` and the desktop-workspace-ui delta for the desired behavior.

## Goals / Non-Goals

**Goals:**

- Give New Project an explicit name-and-location flow and Open Project a folder-selection flow.
- Keep project-only menus inside the workspace and make their lifecycle transitions update one consistent UI surface.
- Keep the core state machine headless and preserve GTK 4.6 compatibility.
- Make dialog cancellation a no-op and keep filesystem/project mutations behind the existing platform boundaries.

**Non-Goals:**

- Redesigning the three-pane editor, preview, or project file navigation.
- Adding recent-project persistence or a new project-management backend.
- Moving long-running project loading to a worker in this fix.

## Decisions

- Use a GTK modal project form for New Project with a name entry, a parent-folder chooser, and Create/Cancel actions. This keeps the project identity explicit instead of inferring it from a selected folder.
- Keep Open Project as a native folder chooser because it is compatible with the supported Ubuntu 22.04 GTK baseline and matches the filesystem project boundary.
- Centralize successful open/create and close widget updates in small UI transition helpers. This prevents button and menu callbacks from diverging and makes the lifecycle observable in tests.
- Build the workspace menu strip as part of the workspace surface and remove duplicate global-header menu controls. Application actions and accelerators remain registered, but their visible menu buttons belong only to the workspace.

## Risks / Trade-offs

- [GTK dialog availability] The native chooser depends on the desktop portal/native GTK implementation → retain the existing GTK 4.6-compatible API and report chooser errors through the status label.
- [Filesystem mutation] Creating a project can partially fail if the selected parent is invalid or non-empty → validate the name/location before calling the platform create boundary and surface the error without switching views.
- [Callback duplication] Multiple menu actions can regress to different state updates → route all successful project lifecycle results through shared transition helpers and add headless state regression coverage.
