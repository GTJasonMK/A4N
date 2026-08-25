---
name: use-a4n
description: Guide or operate A4N through its desktop/Web UI or the `a4n` CLI for A4N-specific project, novel, multimedia, import/export, configuration, collaboration, task, and diagnostic workflows. Use for A4N requests; do not use for generic writing, media generation, or unrelated CLI work.
---

# Use A4N

Choose the interface that matches the request. Load only the relevant references:

- Explain a human workflow with the desktop/Web page path and prerequisites. Read [user-workflows.md](references/user-workflows.md).
- Perform or automate work with the `a4n` CLI. Follow [cli-operation.md](references/cli-operation.md).
- Make an Agent operate the currently visible Web tab and expose only that page's commands. Follow the Page CLI section in [cli-operation.md](references/cli-operation.md).
- Validate the CLI, real Server, Runtime, or virtual Provider path. Read [cli-testing.md](references/cli-testing.md).
- Reuse minimal Agent patterns for project creation, task observation, and artifact export from [agent-recipes.md](references/agent-recipes.md).
- Register and report a long-running Agent job with the shared Server. Follow the Agent lifecycle section in [cli-operation.md](references/cli-operation.md) and the recipe in [agent-recipes.md](references/agent-recipes.md).
- Use output modes, registration/OAuth, local config/runtime snapshots, persistent context, Shell, raw requests, project packages, or media transfer. Read [cli-advanced.md](references/cli-advanced.md).
- Select an end-to-end creation path from [creation-workflows.md](references/creation-workflows.md).
- Handle text creation, worldbuilding, profiles/relationships/events, chapter analysis, shooting scripts, portraits/images, optimization, RAG, and text export with [text-workflows.md](references/text-workflows.md).
- Import manuscripts or project packages, create derived projects, and synchronize source changes with [import-sync.md](references/import-sync.md).
- Produce source-derived video, assemble/post-process a film, or operate the freeform video canvas with [video-canvas.md](references/video-canvas.md).
- Create manga or GALGame content, edit branches/scripts, bind assets, and export WebGAL with [manga-galgame.md](references/manga-galgame.md).
- Configure providers/prompts/themes, diagnose queues/logs, collaborate, or use administrator features with [settings-collaboration.md](references/settings-collaboration.md).

Prefer the ordinary CLI for Server-backed work that is independent of the visible page. Use Page CLI when the Agent must behave like a user on the current Web tab, use page-specific context/commands, or keep the user-visible operation state synchronized. Use the desktop/Web interface for browser-local drafts, drag/drop canvas layout, visual timelines or players, clipboard/native file-picker actions, and requests that explicitly require direct page interaction.

Treat the running CLI manifest and command descriptors as authoritative. Names and examples in these references are routing hints, not permission to guess a payload or bypass a current contract.

Keep progressive disclosure strict. For CLI work, load `cli-operation.md`, then only the business-domain reference and recipe needed for the current task. Do not preload every reference.

When the request is about test coverage or diagnosis, load `cli-testing.md` in addition to `cli-operation.md`; do not infer coverage from descriptor counts alone.

## Establish a safe session

1. Locate the installed `a4n` binary. In a source checkout, build the current CLI once with `cargo build -p a4n-cli`, then use `target/debug/a4n`; do not trust an old binary merely because it exists.
2. For any Server, run `a4n --output agent-json health` and `a4n --output agent-json capabilities check` before writes. Stop if the CLI/Server manifest is incompatible.
3. Run `a4n --output agent-json doctor` only when starting or diagnosing a local deployment; it intentionally checks local PostgreSQL, JWT configuration, runtime binaries, and optional Qdrant state.
4. If local services are not running and local operation is requested, choose the local profile in [cli-operation.md](references/cli-operation.md), verify required environment variables, and start with `a4n local start`. Remember that it does not start PostgreSQL or Docker.
5. Check authentication with `a4n --output agent-json auth status`. Send passwords and OAuth codes through stdin-only auth commands; never place credentials in arguments or output.
6. For a multi-step session, use one explicit `--context PATH`, select the project with `project use`, and select the chapter with `chapter use`. If an Agent is involved, keep the same `--config PATH` (or `A4N_CLI_CONFIG`) for registration and tracked work; context and CLI config are separate files.
7. Register an Agent only when Agent-workbench visibility is needed. `--agent` also disables input and auto-tracks selected work commands, so those commands require a saved lease; without registration use `--output agent-json` for one-shot output and `--output jsonl` for event streams.

## Discover before calling

Use semantic subcommands for auth, project, chapter, task, workflow, context, media, and local lifecycle. For every other operation:

1. If the command is unknown, search the compact catalog with `a4n --output agent-json commands KEYWORD`, using command/domain terms from the relevant reference.
2. Inspect only the selected contract with `a4n --output agent-json capabilities describe COMMAND`.
3. Build arguments from `args_schema`; do not invent fields. If schema quality is `opaque`, inspect the matching frontend API call or make a read-only query first.
4. Invoke with `a4n --output agent-json call COMMAND --args @args.json` (or inline JSON/stdin for small inputs). Use `--agent` here only after registering an Agent for workbench tracking.

Use filtered `capabilities list` only when comparing several descriptors; it includes complete schemas and can be large. Reserve `capabilities export` for compatibility checks, diagnostics, or explicit manifest export.

Use `--output agent-json` for one-shot automation and parse the success/error envelope plus process exit code. Do not infer success from natural-language output.

## Handle long operations

- Capture returned `task_ids`, `workflow_ids`, `project_ids`, `chapter_ids`, and artifact references from the Agent envelope.
- For work performed through `a4n --agent` work commands, a previously registered CLI Agent is inferred as `running`, `completed`, and `failed` from command execution, with an internal heartbeat every 10 seconds. The Server lease is 30 seconds; a missed lease is shown as `offline` in the Tauri/Web Agent workbench.
- The CLI can infer command activity, not hidden model thoughts or work performed outside that process. Use explicit `a4n --agent agent register/update/heartbeat` only for an external worker that owns a long-lived process.
- Use `running`, `paused`, `completed`, or `failed` explicitly. On failure, publish the failure summary before exiting. On normal termination, unregister when the UI no longer needs the Agent entry; a crashed process is marked offline automatically.
- Watch only handles that are still running: use `task watch` for asynchronous generation tasks and `workflow watch` for non-terminal persistent pipelines. Some workflow-recorded calls wait and return a completed result or proposal synchronously; do not watch them again.
- Treat exit code `8` as failed and `9` as cancelled/interrupted. Use `workflow show` before `workflow resume`.
- For repeatable multi-step automation, compose capability-first calls in an ordinary script and persist returned IDs. Do not rely on commands absent from `a4n --help`.

## Handle files and destructive actions

- Upload binary or text inputs with `--set-base64` or `--set-text` rather than embedding large payloads in shell arguments.
- Save exports with `--save-base64`/`--save-text`; download managed media with `media download`.
- Keep atomic output protection. Use `--force` only when the user explicitly wants an existing file replaced.
- Never pass `--allow-destructive` or `--yes` without explicit authorization for that action. Preview/query current state and identify the exact target first.

## Configure only required capabilities

Configure and test LLM first. Add Embedding for RAG/search, image providers for portraits/images/manga, and video providers for video generation. Before starting a generation command, check its `required_capabilities` and use `capabilities check` rather than assuming a provider is available.

## Finish clearly

Report the project/chapter/task/workflow IDs, terminal result, exported file paths, and any step that still requires human review. For UI guidance, state the exact page/button route and warn before irreversible operations.
