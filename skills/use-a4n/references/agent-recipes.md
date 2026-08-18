# Low-context A4N Agent recipes

Load this reference after `cli-operation.md` only when executing one of these patterns. Treat each current command descriptor as the source of truth; the examples define control flow, not fixed business payloads.

## Shared preflight

Use the same Server, config, and explicit context path throughout a multi-step run:

```bash
a4n --agent health
a4n --agent capabilities check
a4n --agent auth status
```

Stop writes when health, compatibility, or authentication fails. Use `doctor` only for a local deployment.

## Create and select a project

List existing projects before creating one when the request may refer to prior work:

```bash
a4n --agent --context /private/path/a4n-context.json project list
a4n --agent --context /private/path/a4n-context.json \
  project create --title '项目名' --project-type novel_long --use-context
a4n --agent --context /private/path/a4n-context.json context show
```

Capture the returned project ID from `meta.references.project_ids` or the response data. Keep using the same context file. Do not create a duplicate project when an existing one satisfies the request.

## Start and observe a generation task

Find a candidate name without loading every schema, then inspect only that command:

```bash
a4n --agent commands chapter
a4n --agent capabilities describe start_chapter_generation
```

Build `start-chapter.json` from `data.args_schema`, including the selected project identity and every current required field. Then invoke it:

```bash
a4n --agent --context /private/path/a4n-context.json \
  call start_chapter_generation --args @start-chapter.json
```

Capture `task_id`, `attempt`, and `status`. If the response is already terminal, consume it directly. Otherwise watch the returned attempt:

```bash
a4n --output jsonl task watch TASK_ID --attempt ATTEMPT
```

On exit code `8` or `9`, inspect task snapshot/history before deciding whether a retry is safe. Reuse or replace request identity only according to the descriptor's `idempotency` contract.

## Export an artifact

Search for the intended format, inspect its file-output pointer, and use the descriptor's current arguments:

```bash
a4n --agent commands export
a4n --agent capabilities describe export_epub_data
a4n --agent --context /private/path/a4n-context.json \
  call export_epub_data --args @export.json \
  --save-base64 /data_base64=book.epub
```

Confirm that the output exists, is non-empty, and matches the expected format before reporting success. Do not add `--force` unless overwriting that exact file was explicitly authorized. Report the absolute output path and relevant project/chapter IDs.
