# Captee

Captee is a local-first desktop workspace for Typst notes and annotated screenshots.

Write Typst with a live preview, capture part of your screen with a shortcut, add an explanation, and insert the image into the focused document.

## Planned features

- Typst source editor with live diagnostics and preview
- Project folders with portable Typst files and PNG assets
- Fast screen-region capture with inline annotations
- PDF export

## Status

Under design.

## Development workflow

The default branch is `master`. Use short, imperative Conventional Commit subjects and keep changes focused. Run the OpenSpec validation command before handoff:

```sh
openspec validate build-captee-v1 --strict
```

Do not commit credentials, local environment files, build output, or tool-specific workspace state.
