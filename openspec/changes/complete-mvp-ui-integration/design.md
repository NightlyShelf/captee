## Context

The GTK adapter currently builds the workspace and dispatches operation-start commands, while core and platform crates contain mostly headless contracts and test doubles for authoring, rendering, capture, assets, export, and persistence. The goal is to connect these boundaries without moving platform effects into core or blocking the GTK loop.

## Goals / Non-Goals

**Goals:**

- Provide one UI-owned operation coordinator that translates widget events into core commands and platform work.
- Run filesystem, compiler, capture, annotation, and export work off the GTK main loop and apply results only for the active project/source revision.
- Wire the real editor buffer, preview, diagnostics, capture/annotation flow, persistence, and settings to existing domain contracts.
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

## Migration Plan

Implement the coordinator and editor persistence first, then preview/export, capture/annotation, and settings/feedback. Each slice keeps the existing shell usable and adds focused tests. Rollback is a normal Git revert because the project format and core public contracts remain backward compatible.
