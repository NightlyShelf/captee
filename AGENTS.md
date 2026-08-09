# Repository Rules

## Priorities

1. Simplicity
2. Speed
3. Safety

Use simple architecture, code, and docs. Follow best practices without overengineering.

## Scope

Do only requested work. Do not add unasked features, docs, abstractions, or refactors.

Keep components logically separate. Prefer small, readable code over complex patterns. Add abstraction only when it removes real duplication or complexity.

## Source of Truth

For planned work, `AGENTS.md`, active OpenSpec changes, and Git history are durable source of truth.

Record every user requirement in the active OpenSpec change when one exists. Before implementation, verify tasks cover all discussed requirements.

## OpenSpec

Use OpenSpec only for large changes. Small scoped changes do not need it.

Keep each change small:

- `proposal.md`: 3–5 sentences explaining requested change and why it matters.
- `tasks.md`: short, exact checklist covering user requests.
- `design.md`: only for large, breaking architecture changes; keep it short.

Do not create unnecessary artifacts or speculate beyond user request.

## Branches and Commits

Use one dedicated branch per feature or bug fix.

Keep commits small, focused, and easy to review. Use short Conventional Commit subjects.

## Code and Tests

Keep code simple, readable, and testable. Separate UI, platform, and core behavior where it helps clarity.

Add focused tests for behavior changes. CI validates code on pull requests. If CI fails, fix it and push a new commit.

## Documentation

Write only useful, requested documentation. Keep every document short, precise, and easy to scan.

Do not maintain architecture logs or duplicate information across documents.

## Safety

Protect local user data. Confirm destructive actions. Cancelled capture or dialog actions must not mutate files or project state.
