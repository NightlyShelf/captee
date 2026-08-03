# Captee

Captee is a local-first desktop workspace for Typst notes and annotated screenshots.

Write Typst with a live preview, capture part of your screen with a shortcut, add an explanation, and insert the image into the focused document.

The implementation is a Rust workspace. See [docs/architecture.md](docs/architecture.md) for crate boundaries and dependency direction.
The pinned toolchain is Rust 1.97.1 with rustfmt and clippy; dependency policy is documented in [docs/dependencies.md](docs/dependencies.md).

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

## Repository access and recovery

Clone the repository with:

```sh
git clone https://github.com/NightlyShelf/captee.git
cd captee
```

For authenticated GitHub operations, use `gh auth login` and verify the active account with `gh auth status`. The canonical remote is `origin`, and the default branch is `main`.

Before opening a change, run `git fetch origin` and rebase or merge from `origin/main` as appropriate. Never force-push `main`. If local metadata is damaged, preserve uncommitted files, clone a fresh copy, and copy only reviewed working files into it; do not delete the remote repository to recover a local checkout.

Releases will be produced from reviewed tags after CI passes. Release credentials belong in GitHub Actions secrets or the platform credential store, never in the repository.

Branch protection is pending a GitHub plan that supports protection rules for this private repository. Until it is enabled, maintainers must avoid direct force-pushes and deletion of `main`; once available, require the `plan-validation` check and conversation resolution on the default branch.
