# A4N creation workflows

Do not hardcode generation payloads. For each named command, run `capabilities describe` and use the current schema.

## New AI-assisted novel

1. Configure/test LLM.
2. Create a text project and select its context.
3. Run inspiration conversations (`start_inspiration_generation`) until the core idea is sufficient.
4. Generate and refine a blueprint (`start_blueprint_generation`, `start_blueprint_refinement`), then confirm it.
5. Generate part/chapter outlines (`start_outline_generation` and the current outline commands).
6. Generate or import chapter text (`start_chapter_generation` or `chapter import`).
7. Analyze chapters and sync character/scene/object profiles.
8. Add Embedding/RAG, images, manga, video, or GALGame only when needed.

Watch each returned handle only when its response/state is non-terminal. Consume synchronous completed results directly.

## Continue an existing manuscript

1. Create/select a text project.
2. Upload TXT, Markdown, EPUB, or DOCX with `import_project_text_upload` and `--set-base64`.
3. Inspect `list_import_queue`; accept intended chapters with `accept_import_review` or `batch_accept_reviews`. Reject/delete only with explicit approval.
4. Run `run_import_project_analysis` and query `get_project_import_analysis_progress` when needed.
5. Preview with `preview_reverse_engineer_request`, then run `reverse_engineer`. It normally waits and returns a proposal plus `workflow_run_id`; inspect the proposal/warnings directly and use `confirm_reverse_engineering_candidate` only after human approval. Watch only if the returned state is non-terminal.
6. Continue in the chapter workflow.

## Derive manga, video, or GALGame

1. Ensure source chapters have saved content.
2. Inspect `create_derived_project`, select kind and chapter range, and create the derived project.
3. Select the new project context.
4. In a video project, create the chapter's director plan and shooting script before planning media. Manga and Galgame create their own adaptation plans from the frozen text; a captured script version from a sibling video is optional.
5. Watch workflows, review generated assets, then export/download.
6. When source text changes, run `preview_source_sync` before `apply_source_sync_decisions`; never apply conflicts blindly.

Each media workspace shows explicit stages. Manga can stop at page planning and Galgame at branch design for review and adoption. Their default one-click text pipelines do not generate paid media. See [manga-galgame.md](manga-galgame.md) for plan versions and optional video references.

## Freeform video canvas

Create with `create_freeform_video_project`, then use canvas graph/node capability descriptors for imports, previews, execution, assembly, and package export. Persist returned IDs and watch only non-terminal task/workflow handles; consume synchronous assembly or preview results directly.

## Recovery strategy

- Task failure: inspect task snapshot/history and error details; retry only after resolving the cause.
- Workflow failure: inspect `workflow show`, checkpoints, and resumability; use `workflow resume` when allowed.
- Contract mismatch: stop writes and update either CLI or Server.
- Missing provider: configure/test only the capability named by `required_capabilities`.
