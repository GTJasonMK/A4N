# Repository Guidelines

## Project Structure & Module Organization

A4N is a Rust workspace under `crates/`, with the React/Vite client in
`frontend/` and operational scripts in `scripts/`. Runtime ownership is split
between `a4n-edge` (HTTP/static proxy), `a4n-control` (API and durable state),
and `a4n-agent-worker` (leased Agent execution). Shared HTTP and worker
contracts live in `a4n-api-contract`, `a4n-service-protocol`, and
`a4n-worker-contract`; keep business rules in their existing owner instead of
duplicating them in launchers or routes.

## Build, Test, and Development Commands

Requires Rust, Node.js 22, npm, and PostgreSQL 16 or newer. Use the repository
scripts for normal workflows:

- `./scripts/dev/dev-db.sh up` starts the development database.
- `./scripts/run.sh` starts Tauri development; pass `--server` for web mode.
- `./scripts/build.sh --check` checks Rust compilation; without `--check` it
  builds the application.
- `timeout 60s ./scripts/test.sh --crate a4n-core --nocapture` runs targeted
  backend tests when the crate is applicable.
- `cd frontend && npm run test:run` runs Vitest, and `npm run build` checks the
  web bundle.

## Coding Style & Naming Conventions

Run `cargo fmt --all -- --check` and `git diff --check`. Rust uses four spaces,
`snake_case` functions/modules, and `PascalCase` types. TypeScript/TSX uses two
spaces, double quotes, semicolons, `camelCase` functions, and `PascalCase`
components. Read callers before changing a shared boundary and keep contracts
single-sourced.

## Testing Guidelines

Rust unit tests use `#[test]` or `#[tokio::test]` beside the module or in
`tests/`; frontend tests use `*.test.ts` and `*.test.tsx`; browser tests live
under `frontend/e2e/`. Test changed invariants and failure paths. Backend test
commands default to a 60-second timeout. Database tests require an isolated
`A4N_TEST_DATABASE_URL`; never treat skipped or timed-out checks as passing.

## Commit & Pull Request Guidelines

Use Conventional Commits such as `refactor:`, `fix:`, `feat:`, `test:`, and
`docs:`. PRs must describe ownership or behavior changes, link issues when
available, record contract or migration changes, list actual validation, and
state remaining risks. Include screenshots for visible UI changes.

## Safety & Collaboration

Check `git status --short` first and preserve unrelated work. Keep secrets in
ignored environment configuration; do not commit `.env`, databases, logs, or
build artifacts. Validate boundary inputs, parameterize SQL, and use
`docs/work.md` when coordinating shared work if that file exists.
