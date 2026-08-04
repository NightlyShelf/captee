# Captee

Captee is a local-first desktop workspace for Typst notes and annotated screenshots.

Write Typst with a live preview, capture part of your screen with a shortcut, add an explanation, and insert the image into the focused document.

The implementation is a Rust workspace. See [docs/architecture.md](docs/architecture.md) for crate boundaries and dependency direction.
The pinned toolchain is Rust 1.97.1 with rustfmt and clippy; dependency policy is documented in [docs/dependencies.md](docs/dependencies.md).
The first Linux distribution bundles Typst through the verified procedure in [docs/third-party-typst.md](docs/third-party-typst.md).
GitHub Actions runs pinned rustfmt, clippy, full workspace tests, and UI-state tests on pushes and pull requests to `main`. AppImage packaging is intentionally manual-only from the `AppImage package` workflow.

## Implemented vertical slices

- Typst source editor with revision-aware diagnostics and preview adapters
- Project folders with portable Typst files and PNG assets
- Fast screen-region capture with inline annotations
- PDF export

## Project layout

- `crates/captee-core`: headless project, editor, render, capture, and application-state logic
- `crates/captee-platform`: filesystem, Typst, capture, export, and trash adapters
- `crates/captee-ui`: GTK 4 desktop shell and GtkSourceView editor integration
- `docs/`: architecture, dependency, recovery, runtime, and release notes
- `tools/`: pinned third-party download and verification helpers

## Development commands

Install Rust 1.97.1 and the GTK development libraries before building the UI:

```sh
sudo apt-get install libgtk-4-dev libgtksourceview-5-dev
```

Run the complete local gates:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Run the desktop shell with a Wayland or X11 session:

```sh
cargo run -p captee-ui
```

Release packaging inputs and the AppImage procedure are documented in
[docs/release.md](docs/release.md) and [packaging/appimage/README.md](packaging/appimage/README.md).
To create a package, open GitHub Actions, select `AppImage package`, and use
`Run workflow`; ordinary pushes and pull requests never build an AppImage.

## Development workflow

The default branch is `main`. Use short, imperative Conventional Commit subjects and keep changes focused. Run the OpenSpec validation command before handoff:

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
