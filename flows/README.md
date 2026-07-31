# A4N CLI Flows

Flow 是给 Agent 使用的可恢复 JSON 操作序列。所有业务步骤仍通过 `a4n-server`
进入共享 runtime/orchestrator；Flow 只负责参数引用、危险操作授权、journal 和恢复。
完整 CLI、输出合同和退出码说明见 [`docs/CLI.md`](../docs/CLI.md)。

常用命令：

```bash
a4n flow list
a4n flow validate flows/project-list.json
a4n --agent flow run flows/project-list.json --journal /tmp/a4n-project-list.journal.json
a4n --agent flow resume /tmp/a4n-project-list.journal.json
```

占位符：

- `${context.project_id}`、`${context.chapter_id}`、`${context.snapshot_id}`
- `${env.NAME}`：敏感值必须通过环境变量提供，不得写入 Flow JSON
- `${steps.<step-id>#<json-pointer>}`：引用已完成步骤保存的结果
- `${run.id}`：当前 Flow run UUID

步骤可用 `capture` 只把后续需要的 JSON Pointer 写入 journal。危险命令必须在
spec 中设置 `allow_destructive: true`，执行时还必须传 `--allow-destructive`。

支持的 step 类型：

- `call`：调用共享 Host API。
- `task-watch`：观察 generation task 到终态。
- `workflow-watch`：观察持久化 workflow 到终态。
- `media-download`：原子下载 Server 媒体文件。

`resume` 可设为 `auto`、`rerun` 或 `manual`。默认 `auto` 会读取 command
descriptor 的幂等性；无法证明幂等的中断步骤必须在确认服务端状态后使用
`--rerun-incomplete`。Flow 源文件发生变化时会拒绝从旧 journal 自动恢复。
