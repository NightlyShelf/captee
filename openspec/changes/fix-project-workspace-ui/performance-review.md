# Performance review

## Task scope

The review covers the GTK project dialogs, project lifecycle transition helpers, and workspace menu-strip changes in this change.

## Findings

| Area | Finding and impact | Mitigation | Follow-up |
| --- | --- | --- | --- |
| I/O / UI responsiveness | Creating or opening a project reads configuration and the entry document synchronously in the GTK callback. Large entry documents can briefly block the main loop. | Keep the operation limited to an explicit user action and retain the existing small-project behavior; report failures without changing the visible surface. | Move project loading to a bounded worker when large-project support is added. |
| Memory | Opening a project holds the loaded source string while copying it into the GtkSourceView buffer. Peak memory is approximately one entry-document copy during the transition. | Do not retain a second application-owned source copy after the buffer update. | Measure large-document behavior before introducing background loading. |
| Concurrency / lifetime | Dialog callbacks retain GTK widgets and project UI state until the user responds. A cancelled chooser must not mutate project state. | Use modal dialogs with response callbacks and only dispatch/open or create after validated acceptance; close the dialog after successful transition. | Add asynchronous loading only with revision/lifetime guards. |
| Process / external I/O | The fix does not add subprocesses or background processes; native folder selection remains delegated to GTK. | Preserve the GTK 4.6-compatible native chooser and avoid unbounded polling or child-process lifetime. | None for this change. |
