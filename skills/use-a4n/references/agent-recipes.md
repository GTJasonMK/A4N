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

## Register and observe an external Agent

Use this around a long-running CLI or automation job so the Tauri/Web Agent workbench can show the current phase. The registration command persists the lease token locally; subsequent commands can omit `--agent-id`.

For ordinary commands launched by an A4N Agent, register once and prefer `a4n --agent ...`: the CLI then infers the phase, heartbeats every 10 seconds, and records completion/failure. This explicit recipe is for a worker that runs outside that command lifecycle.

```bash
a4n --agent health
a4n --agent capabilities check
a4n --agent auth status

a4n --agent agent register \
  --name '上一章视频转换器' \
  --kind media \
  --client cli \
  --project-id 42 \
  --chapter-id 3

a4n --agent agent update \
  --status running \
  --phase '准备素材' \
  --summary '读取上一章和角色设定' \
  --progress 0
```

During the job, update the visible state after meaningful phase changes and refresh the lease every 20 seconds or less:

```bash
a4n --agent agent heartbeat --status running --phase '生成关键帧' --progress 40
a4n --agent agent heartbeat --status running --phase '合成视频' --progress 80
```

Finish with an explicit terminal state. Publish failure context before returning a failed process. Leave the completed entry for UI review or unregister it when it should disappear:

```bash
# success
a4n --agent agent update --status completed --progress 100 --summary '视频已生成'
# failure
a4n --agent agent update --status failed --summary '视频供应商超时，未生成成片'
# optional cleanup
a4n --agent agent unregister
```

If the process crashes, no cleanup command is needed: the Server marks the Agent `offline` after its 30-second lease expires. The registry is runtime-only, so restart/resume history belongs in the normal task/workflow APIs.

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
