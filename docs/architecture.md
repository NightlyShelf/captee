# Architecture

Captee is organized as a Cargo workspace with three crates:

| Crate | Responsibility | Constraints |
| --- | --- | --- |
| `captee-core` | Project models, editor state, revision handling, validation, and pure services | Must remain headless and free of GTK, Linux, process, and real-filesystem dependencies |
| `captee-platform` | Filesystem, atomic persistence, bundled Typst, portal/capture, trash, and subprocess adapters | Depends on core; platform effects are exposed through narrow interfaces and test doubles |
| `captee-ui` | GTK 4 desktop shell, commands, state presentation, and accessibility integration | Depends on core and platform; widgets do not own project or process side effects |

The dependency direction is intentionally one-way: UI → platform → core. Core
tests must be runnable without a desktop session, GTK development packages, or
the bundled Typst executable. Long-running compiler, renderer, and capture work
will be scheduled outside the GTK main loop and applied through revision-checked
results.

## Performance considerations

The initial source editor stores complete text snapshots for undo and redo. This
keeps the implementation simple and reliable for ordinary notes, but memory use
can grow approximately with document size multiplied by the number of edits.
Large documents or long editing sessions may therefore become a bottleneck.

Before optimizing for large documents, history should move to bounded edit
records containing ranges and inserted/removed text, coalesce continuous typing,
and enforce a byte budget. Periodic full snapshots can remain as recovery and
fast-replay checkpoints. Any replacement must preserve the same undo, redo, and
revision semantics covered by the core tests.
