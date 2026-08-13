## Why

Typst authoring needs accurate IDE-style assistance without duplicating language intelligence already provided by Tinymist. Project navigation and command menus also become awkward as projects grow, while exiting with unsaved work leaves an autosave that triggers an avoidable recovery prompt on the next run.

## What Changes

Bundle pinned Tinymist 0.14.6 and use its LSP for completion, diagnostics, and a compact native-looking editor error count while retaining the existing Typst 0.14.2 compiler for formatting, preview, and export. Improve workspace navigation with a vertically scrollable, initially collapsed project tree; keep the caret and editor viewport on the latest edit; follow that edit in the preview; place Capture and Settings under Edit, Export PDF under File, and expose About as its own menu button. Prompt to Save, Discard, or Cancel before dirty exit, clear recovery data only after Save or Discard succeeds, align Captee metadata with GPL-3.0-or-later, and retain bundled Typst and Tinymist Apache-2.0 licenses and notices.
