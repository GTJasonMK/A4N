# A4N CLI 高级入口

只有需要脚本化、诊断或跨命令复用状态时才加载本页。业务调用仍遵循
`commands → capabilities describe → call`，不要把本页示例当成业务 schema。

## 目录

- 输出与 Agent 合同
- 注册、OAuth 与凭证
- 命令目录与 manifest
- 配置、运行时快照与 context
- 交互 Shell 与原始 request
- 项目包、媒体与文件指针
- 自动化边界

## 输出与 Agent 合同

根级 `--output` 要放在子命令前：

```bash
a4n --output human project list
a4n --output json project list
a4n --output jsonl task history
a4n --output agent-json capabilities export
```

- `human` 适合终端查看；`json` 是单个 JSON 文档；`jsonl` 把数组或事件逐行输出。
- `agent-json` 返回稳定的 `{ok, command, data, meta}` envelope；失败 envelope 写到 stderr。
- `--agent` 等价于 `--output agent-json`，并关闭交互和颜色；此外它会为部分工作命令启用 Agent 生命周期跟踪，必须已有保存的 Agent 租约。没有跟踪需求时用 `--output agent-json`；`--compact` 只影响普通 JSON。
- 自动化同时检查 envelope 和退出码；不要从自然语言输出猜测成功。

## 注册、OAuth 与凭证

密码、密码变更内容和 OAuth code 只能走 stdin，不能放在参数、脚本文件或日志中：

```bash
printf '%s\n' "$A4N_PASSWORD" |
  a4n auth register --username writer --email writer@example.com \
    --invite-code INVITE --password-stdin

a4n auth oauth-authorize github
printf '%s\n' "$OAUTH_CODE" | a4n auth oauth-callback github --code-stdin
```

常用会话入口还有 `auth status`、`auth refresh`、`auth sessions`、
`auth revoke-session SESSION_ID --yes`、`auth update-profile` 和 `auth logout`。
`A4N_ACCESS_TOKEN` 是只读环境认证来源；它存在时不要执行 login/register/refresh/logout，
先在父进程取消该变量。`a4n call` 会拒绝直接调用认证命令，避免绕过本地会话管理。

## 命令目录与 manifest

```bash
a4n --output agent-json commands
a4n --output agent-json commands video
a4n --output agent-json capabilities describe start_video_generation
a4n --output agent-json capabilities list --domain video
a4n --output agent-json capabilities export
a4n --output agent-json capabilities check
```

`commands` 只列出 CLI 命令和桌面运行时等价入口，适合低上下文发现；`describe` 返回单个
业务命令的参数/结果 schema、effect、execution、文件指针、所需能力和幂等性。
`capabilities list` 会返回匹配命令的完整 schema，仅在比较多个候选时使用；`export` 只用于
显式导出或诊断。`check` 返回退出码 `11` 时停止写操作，先让 CLI 与 Server 的 manifest 对齐。

## 配置与运行时快照

```bash
a4n config show
a4n config set-server https://example.com/a4n
a4n config clear-auth
a4n config runtime show
a4n config runtime set --input @runtime-config.json
a4n config runtime clear
```

`config show` 不输出 token；`clear-auth` 只清理本地存储，不会清除父进程中的
`A4N_ACCESS_TOKEN`。runtime snapshot 会按 descriptor 的 `None`、`WithoutSecrets` 或
`Full` 策略自动注入 `call` 请求。`runtime show` 会隐藏密钥，但持久化配置文件和 set
输入可能含 API key，必须限制访问且不要提交。

`--config PATH` 或 `A4N_CLI_CONFIG` 选择保存 token、Server 地址、runtime snapshot 和
Agent 租约的 CLI 配置；`--context PATH` 是独立的项目/章节上下文文件。注册 Agent 和
后续 `--agent` 工作命令必须使用同一 CLI 配置。

## 跨命令 context

```bash
CTX=/path/to/a4n-context.json
a4n --context "$CTX" project use 42
a4n --context "$CTX" chapter use 100
a4n --context "$CTX" context show
a4n --context "$CTX" context set --project-id 42 --chapter-id 100
a4n --context "$CTX" context clear-snapshot
```

还可用 `context clear`、`clear-project`、`clear-chapter`。切换项目会清理章节和快照，
切换章节会清理快照；持久化 context 绑定 Server 与用户，发现归属变化时拒绝覆盖。
没有 `--context` 时，context 只在当前进程（包括交互 Shell）内有效。Unix 可使用
`${XDG_CONFIG_HOME:-$HOME/.config}/a4n/context.json`；Windows 使用
`%APPDATA%\\A4N\\context.json` 或其他显式安全路径。

## 交互 Shell

```bash
a4n shell
a4n shell --no-history
a4n shell --history /path/to/history
```

Shell 与 one-shot 共用解析器，支持 `help [command ...]`、补全和 `exit`/`quit`；不执行
shell expansion 或命令替换，敏感命令不会写入 history。Shell 不能与 `--agent` 或
`--no-input` 一起使用；Server、配置、context 和输出模式在会话内固定。

## 原始 request（仅诊断）

```bash
a4n request get /meta --no-auth
a4n request post /commands/some_command --body @request.json \
  --allow-unsafe-request
a4n request get /meta --output ./meta.json
```

`request` 不读取业务 manifest，也不识别业务 destructive 语义；原始非 GET（POST/PUT/
PATCH/DELETE）必须显式 `--allow-unsafe-request`。优先用 typed `capabilities` + `call`；
只有排查 Host API 或尚未纳入 manifest 的只读接口时才使用 request。文件输出默认拒绝
覆盖，需同时传 `--force`。

## 项目包、媒体与文件指针

项目族/派生项目/视频项目包使用当前 manifest 中的命令：

- `export_project_family_data` ↔ `import_project_family_upload`
- `export_derived_project_data`
- `export_video_studio_project_data` ↔ `import_video_studio_project_upload`
- `import_canvas_media`（自由画布媒体）

导出用 `--save-base64 /data_base64=FILE` 或 `--save-text /field=FILE`，导入用
`--set-base64 /input/data_base64=FILE` 或 `--set-text /input/field=FILE`；默认原子写入且
拒绝覆盖。Server 返回的相对媒体路径用 `a4n media download PATH --output FILE` 下载，
需要覆盖时再加 `--force`。导入包后重新确认项目 ID、来源快照、章节范围和媒体绑定，
不要复用包内旧 ID。

## 自动化边界

当前 CLI **没有 `a4n flow` 子命令**。旧流程文档中的 `flow run/resume` 不可执行；自动化
请用普通脚本顺序组合 `--output agent-json call`、task/workflow 观察和媒体下载，并持久化返回的 ID。只有已注册 Agent 且需要工作台可见状态时才改用 `--agent`。
重试非幂等调用前先查 descriptor 的 `idempotency` 和 Server 当前状态。
