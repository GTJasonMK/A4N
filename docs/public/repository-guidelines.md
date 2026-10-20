# Repository Guidelines

A4N combines Rust/Tauri desktop and axum Web hosts with React/TypeScript. This quick reference complements the detailed [contribution policy](../../CONTRIBUTING.md).

## Project Structure & Module Organization

- `crates/<domain>/<a4n-package>/`: Rust implementations in `src/`, integration tests in `tests/`. `a4n-orchestrator` owns workflows, `a4n-db` persistence, `a4n-runtime` adapters, and `a4n-api-contract` Host contracts.
- `frontend/src/`: pages, shared components, typed `api/` wrappers, and `assets/`; browser tests live in `frontend/e2e/`.
- `prompts/`: runtime templates. `scripts/`: development, contract generation, checks, and E2E tooling. `docs/public/`: maintained guides.

## Build, Test, and Development Commands

Complete the [setup guide](getting-started.en.md): Rust, Node.js 22, npm, PostgreSQL 16+, and platform/ONNX/Qdrant dependencies. Run from the appropriate worktree root; keep builds and services out of the `master` coordination directory.

| Command | Purpose |
| --- | --- |
| `npm --prefix frontend ci` | Install locked dependencies. |
| `./scripts/dev/dev-db.sh up` | Start development PostgreSQL. |
| `./scripts/run.sh` | Start desktop development. |
| `./scripts/run.sh --server` | Start Web Server development. |
| `./scripts/build.sh` | Build frontend and backend. |
| `./scripts/build.sh --backend --check` | Check backend compilation. |
| `npm --prefix frontend run build` | Type-check and bundle frontend. |
| `npm --prefix frontend run test:run` | Run Vitest. |
| `npm --prefix frontend run check:web-api-parity` | Check generated contracts. |
| `timeout 60s ./scripts/test.sh --crate a4n-domain` | Test the selected Rust crate. |

## Coding Style & Naming Conventions

Rust uses four spaces, `snake_case` functions/modules, `PascalCase` types, and rustfmt (`cargo fmt --all -- --check`). TypeScript/TSX uses two spaces, double quotes, semicolons, `camelCase` functions, and `PascalCase` components/types. Regenerate contracts from their source; reuse shared business rules.

## Testing Guidelines

Keep Rust `#[test]`/`#[tokio::test]` tests near modules; colocate Vitest `*.test.ts`/`*.test.tsx` files. Browser E2E uses WebdriverIO/Mocha. Cover changed invariants and failure paths. Use isolated `A4N_TEST_DATABASE_URL`; serialize schema-changing tests. Workflow/API changes require corresponding mock-provider CLI E2E; UI changes require actual desktop/narrow-screen browser verification. Run `git diff --check`.

## Commit & Pull Request Guidelines

Use Conventional Commits such as `fix:`, `refactor:`, `feat:`, and `docs:`. Develop in `agent/<topic>` worktrees based on `debug`; follow local coordination/build-lock rules and validated `integration` handoff.

Follow the [PR template](../../.github/PULL_REQUEST_TEMPLATE.md): explain the problem, behavior, related issues, contract/migration changes, actual checks, and remaining risks. Attach screenshots for visible UI changes.

## Security & Configuration

Use ignored `.env` files or environment variables; isolate databases, ports, and runtime data per worktree. Never commit credentials, databases, logs, or generated media. See [configuration](configuration.en.md).
