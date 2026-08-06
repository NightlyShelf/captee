## Context

The GTK adapter currently provides an end-to-end editor, preview, export, and capture workflow, but capture confirmation is not yet a document-aware composition step and the project panel is not an IDE-style file tree. The next increment must extend the existing boundaries without moving platform effects into core or blocking the GTK loop.

## Goals / Non-Goals

**Goals:**

- Provide one UI-owned operation coordinator that translates widget events into core commands and platform work.
- Run filesystem, compiler, capture, annotation, and export work off the GTK main loop and apply results only for the active project/source revision.
- Wire the real editor buffer, preview, diagnostics, capture/annotation flow, persistence, and settings to existing domain contracts.
- Keep the selected capture visible after region selection and let the user review its exact Typst insertion context before any project mutation.
- Share one Typst language-service presentation path between the main editor and capture annotation editor.
- Make project navigation and file operations discoverable while preserving project-root validation and confirmation for destructive operations.
- Make the workspace geometry and menu treatment stable and predictable across window sizes.
- Keep every external effect behind narrow adapters that can be replaced by test doubles.

**Non-Goals:**

- Changing the core state model into a GTK-dependent model.
- Adding collaboration, cloud storage, a plugin system, or a second document format.
- Replacing the existing portal/fallback strategy or project file format unless integration exposes a concrete contract defect.

## Decisions

- Add a UI-side coordinator holding the active project context, source revision, editor buffer bridge, render state, and operation handles. It owns GTK callbacks and sends typed results back through the main context.
- Use worker threads plus GLib main-context callbacks for preview, capture, persistence, and export. Each result carries a project identity and source revision; stale results are discarded before touching widgets or core state.
- Adapt the GtkSourceView buffer to `SourceDocument` and `DocumentPersistence` through a narrow bridge. Save and autosave use the existing atomic platform primitives, while the UI owns dirty indicators and recovery prompts.
- Build capture as a staged pipeline: backend selection, optional annotation editing, confirmation, asset storage, then insertion. Cancellation exits before storage or source mutation.
- Keep menu action registration centralized but route each action to a concrete coordinator operation. Busy/cancellable state controls visible progress and action sensitivity rather than leaving the state machine permanently running.
- Use existing `RenderState`, `AsyncPreviewCompiler`, and `export_pdf` as the preview/export boundary; the UI renders PDF output through the workspace preview adapter and uses a native destination chooser for export.

## Risks / Trade-offs

- [Thread-to-GTK lifetime] A late worker result could update a closed project or destroyed widget → use weak widget references plus project/revision tokens and cancel/join operation handles on close.
- [Capture backend availability] Portals and compositor tools differ across Linux desktops → preserve portal-first behavior, expose configured fallback errors, and test both completion and cancellation with doubles.
- [Large documents/images] Copying source and decoded PNG data can create temporary memory peaks → keep one bounded in-flight operation, enforce existing image limits, and avoid retaining duplicate buffers after application.
- [Persistence failure] Save/autosave failures can desynchronize dirty state → clear dirty only after atomic success and retain the last valid source/project state on failure.
- [Scope size] Full integration spans multiple existing capabilities → implement and verify in task-sized vertical slices with an active performance review after each slice.
- [Capture review state] A preview can accidentally imply a source mutation before confirmation → keep the capture, rendered context, and proposed insertion as staged state; only confirmation stores the image and edits source.
- [Drag-and-drop ambiguity] A drop can target a file, itself, or a path outside the project → validate source and destination as project-relative paths, reject invalid/self-descendant moves, and leave the tree unchanged on failure.
- [Editor language-service duplication] Separate syntax and completion implementations can diverge → use one shared Typst language-service adapter with editor-specific cursor and buffer bridges.
- [Responsive layout] Fixed proportions can become unusable in small windows → use the requested one-sixth project allocation as the initial allocation and preserve minimum child sizes while maintaining a 50/50 remainder split where space permits.

## Migration Plan

The completed foundation remains in place. Implement the capture review composer, shared Typst language assistance, project tree interactions, and layout/menu polish as separate slices. After each slice, run focused tests and an on-screen smoke check, then record the slice performance review and architectural consequences. Defer Clippy until all new slices are implemented; final formatting, Clippy, tests, OpenSpec validation, and the complete visual workflow run are the last verification gate. Rollback is a normal Git revert because the project format and core public contracts remain backward compatible.
