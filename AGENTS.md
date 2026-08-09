# Repository Guidelines

## Project Structure & Module Organization

This repository is currently a bootstrap workspace: no application source, test suite, or package manifest is present yet. Keep the root reserved for project configuration and top-level documentation. As the project is introduced, use a predictable layout such as `src/` for application code, `tests/` for automated tests, `assets/` for static files, and `docs/` for longer-form documentation. Keep a feature's code and related tests close together when that makes navigation clearer.

## Build, Test, and Development Commands

No build or test command is configured at present. When adding a toolchain, document the canonical commands in `README.md` and expose them through the ecosystem's standard entry point (for example, `npm run dev`, `npm test`, and `npm run build`, or their equivalent). Avoid relying on undocumented local setup steps.

## Coding Style & Naming Conventions

Follow the formatter and linter selected for the project; add their configuration files to version control. Use 2 spaces for JSON, YAML, and JavaScript/TypeScript unless the chosen formatter specifies otherwise. Name files and directories consistently—prefer `kebab-case` for general files and use the language's normal conventions for source symbols (for example, `PascalCase` components and `camelCase` functions in TypeScript).

## User Interface Code

Write UI code that is clean, readable, and easy to maintain. Keep presentation, state, and platform integration concerns clearly separated; prefer small, purpose-named components and functions over large, mixed-responsibility views. Preserve accessibility and clear interaction states while avoiding unnecessary abstraction.

## Development Practices

Keep UI code independent from project, Typst, capture, and filesystem logic. Make core behavior testable without GTK, the desktop environment, or real filesystem side effects; wrap platform capture, trash, and external-tool interactions behind narrow interfaces with test doubles. Run Typst compilation asynchronously with debouncing and discard stale results before updating the UI. Protect user work with atomic autosaves, confirmation before moving project items to trash, and no mutation when a capture is cancelled. Add focused core-logic tests for every behavior change and run formatting, linting, and the complete test suite before handoff.

## OpenSpec

Keep OpenSpec small and direct. Proposal: 3–5 precise sentences. Tasks: short, exact checklist. Create change `design.md` only for complex architecture; prefer simplest design. Record every user-stated change requirement in proposal, specs, or tasks. Before implementation, verify artifacts cover all discussed requirements. Validate the change before handoff or archive. Do not maintain a root architecture log.

After completing each implementation task, stop and ask the user whether to continue. Do not commit, push, or begin the next task until the user confirms continuation. After confirmation, commit the completed task, push the commit to the repository, wait for the CI results, and only then move to the next task. If CI fails, stop and report the failure before continuing.

## Testing Guidelines

Add automated tests with every behavior change once a test framework is selected. Place test files under `tests/` or beside the module using the framework's conventional suffix, such as `*.test.ts`. Cover normal behavior, validation, and regressions; run the complete suite before opening a pull request.

## Commit & Pull Request Guidelines

Git history is not available in this workspace, so no established commit convention can be inferred. Use short, imperative commit subjects according to coventional commits style, such as `feat: add caption export flow`. Keep commits focused. Pull requests should explain the change, note validation performed, link relevant issues, and include screenshots or recordings for user-visible changes.

## Configuration & Secrets

Never commit credentials or local environment files. Provide a checked-in example such as `.env.example` for required configuration, with safe placeholder values and clear variable descriptions.

## Implementation Branch and Merge Policy

For every implementation change, work on a dedicated feature branch and do
not merge it until all planned tasks, tests, and required OpenSpec artifacts
are complete. Keep implementation branches available for review until the
change is verified end to end.

For the `cache-rust-gtk-docker-images` change specifically, keep the test and
build container images separate, consume only immutable image digests in
workflows, and preserve the manual-only AppImage trigger.
