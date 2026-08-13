## Boundaries

`captee-platform` owns a small Tinymist process and LSP transport boundary; `captee-ui` owns popup, text-edit, underline, hover, and exit-dialog presentation. `captee-core` remains headless and independent of GTK, child processes, and LSP types. Existing Typst compiler paths remain unchanged.

## Tinymist lifecycle and documents

Start one `tinymist lsp` child for each active project and exchange `Content-Length` framed JSON-RPC over stdio from background threads. Initialize with the project root, open the real entry-document URI plus a stable non-writing capture-review URI under that root, synchronize monotonically increasing document versions, then send shutdown/exit and terminate the child if graceful shutdown fails. Tag requests and notifications with document identity and version so UI ignores stale completion and diagnostic work; failure disables only editor assistance and reports status.

## Packaging and licensing

Pin Tinymist 0.14.6 because it embeds Typst 0.14.2, matching Captee's compiler. Fetch only the selected x86_64 Linux release archive, verify its recorded SHA-256 digest, and retain upstream Apache-2.0 license and notices beside the existing Typst files. Captee package metadata uses GPL-3.0-or-later; third-party components keep their own licenses and notices.

## Dirty exit

Intercept application close before GTK destroys the window. Clean state exits immediately; dirty state waits for Save, Discard, or Cancel. Save exits only after atomic persistence and autosave cleanup succeed, Discard clears only the autosave before exit, and failure or cancellation keeps the current session open.
