# Agent WP1/WP2 详细设计：上下文工程与工具契约

> 状态：设计稿，仅对应 `debug` 分支 `d1e1cdfc`，本文件不改变运行时代码。
>
> 范围：WP1「接入已有上下文工程层」与 WP2「工具观察契约、错误可诊断性、工具窗口治理」。
>
> 非目标：本稿不重写 Agent 状态机、租约、审批、幂等、worker/control 拓扑，也不引入 MCP/A2A/ANP。

## 1. 决策摘要

### 1.1 重构原则

A4N 不照搬 Hello-Agents 的 Python 框架。Hello-Agents 给出的是可迁移的形状：

- Agent 状态必须显式，终止原因必须显式。
- 工具错误是下一轮可以消费的数据，不是丢失上下文的异常字符串。
- 上下文是有限的注意力预算，必须经过 Gather、Select、Structure、Compress。
- 工具描述是模型决策的一部分；工具越多不一定越好。
- 可诊断性先于一次性完成，失败轨迹必须保留。

A4N 已经有更强的持久化执行能力，因此本次只补接线和契约：

```text
现有 AgentApplication 状态机
        |
        v
现有 Agent runtime loop
        |
        +-- WP1: a4n-llm-context 的 plan/manifest/trace 接线
        |
        +-- WP2: model observation envelope + tool window policy
        |
        v
现有 execution/message/event/approval/lease 提交
```

### 1.2 必须保持的事实

1. `AgentRuntimeHost` 仍是领域副作用的边界；WP1/WP2 不把数据库或领域 ORM 放进 runtime。
2. `agent_execution.result_json` 仍保存完整的 Host 原始结果；给模型看的观察结果可以受限、摘要化或带分页引用，但不能覆盖完整结果。
3. `execution_key`、lease fencing、`outcome_unknown`、审批消费规则和 recovery terminal 状态保持不变。
4. 工具可见性、scope、capability、phase、approval 是代码闸门，不由 Skill、上下文 pack 或模型文字决定。
5. 外部业务上下文继续按需通过工具取得。WP1 不把项目正文、章节正文或画面方案自动塞进每一轮 prompt。
6. 已存在的 `a4n-llm-context` 是上下文预算与 manifest 的唯一 owner；不新建第二套 `ContextBuilder` 或 token 估算器。

### 1.3 先修的独立阻塞项

`crates/a4n-db/src/agent/generation.rs:10,12,68,71` 仍按已经删除的 `send_content_optimization_message` 和 `generation_start_request_id` 做 generation owner JOIN，而 debug 的写入键是 `generation_task_id` 与 `generation_attempt`（同文件约 106 行）。

这不是 WP1/WP2 的设计内容，但应在代码实施前修复，否则 Agent 预算归属、用户新输入检测和 owner 停止后的生成任务取消都可能静默失效。修复要单独提交、单独验证，不在本稿中混入上下文改造。

## 2. debug 当前实现事实

### 2.1 Agent 主循环

`crates/a4n-agent-runtime/src/engine/mod.rs` 的 `process_claimed_run_inner`（约 799 行）目前执行：

1. 装载 `AgentRunContext`、runtime snapshot、消息和 inbox。
2. 恢复已经持久化的 model decision 和 pending tool batch。
3. 从 execution 记录重建 steps、model calls、tokens、discovery cache。
4. 调用 `available_runtime_tools` 和 `search_agent_skills`。
5. 用 `with_agent_system_prompt` 组装 system message。
6. 用 `history::fit_context_projection`（约 1076 行）按消息单元裁剪历史。
7. 在输入达到容量 80% 且存在被省略历史时调用 `compact_context`。
8. 构造 `ChatRequest`，持久化 model execution，调用 `chat_stream`。
9. 解码 final/think/tool calls，持久化 assistant message，执行工具批次。

模型 trace 当前明确声明了 `workflow = "agent"`，但：

```rust
context_plan_json: None,
context_pack_refs_json: None,
context_manifest_json: None,
```

对应位置是 `engine/mod.rs:1179-1192`。

### 2.2 已有的上下文 owner

`crates/a4n-llm-context` 已经提供：

- `LlmContextManifest`：`schema_version`、pack 列表、动态任务内容、输出契约。
- `LlmContextManifestPack`：scope、pack kind、source refs、config hash、required、stable prefix、variants。
- `LlmContextPackRequirement`：一个 pack 的 required/stable/variants 描述。
- `ContextBudgetPlanner::plan_requirements`。
- `ContextBudgetPlanner::plan_requirements_adaptive`：必要时二分搜索可用 output reserve。
- `LlmContextPlan`：容量、输出预留、安全余量、stable/dynamic token、压缩档位、选中 pack、被丢弃 optional pack、估计输入 token。
- `render_context_manifest_user_content`：把 stable pack、output contract、variable pack、当前任务按固定结构渲染。
- `estimate_context_tokens`：当前 workspace 的统一估算入口。
- `COMPRESSION_FULL_RAW`、`COMPRESSION_FOCUSED_RAW`、`COMPRESSION_COMPRESSED_SCENE`、`COMPRESSION_MINIMAL` 四档。

特别是 `PlannedContextPack.stable_prefix` 与 manifest renderer 已经区分稳定前缀和可变上下文。`llm_call_trace` 也已有：

```rust
context_plan_json: Option<Value>,
context_pack_refs_json: Option<Value>,
```

`a4n-runtime/src/dynamic_llm/trace/recording.rs:20-41` 会把 request 的 trace context 原样写入 `llm_call_trace`。后台 analytics 已经在 `a4n-db/src/admin/generation_analytics/metrics.rs:83-92` 读取：

- `stable_prefix_tokens`
- `dynamic_delta_tokens`
- `estimated_input_tokens`
- `dropped_optional_pack_count`

因此 WP1 的主要工作是接线，而不是再造上下文引擎。

### 2.3 当前观察结果与工具窗口

`history::bounded_observation`（`engine/history.rs:19-33`）已经做到：

- 完整结果留在 execution。
- 超过 `16 * 1024` bytes 时只给模型一个可回查 stub。
- 通过 `read_agent_execution_result` 按 execution id、JSON pointer、offset、limit 分页读取。

`execute_tool_calls`（`engine/mod.rs:1755-1863`）的并发规则已经明确：

- 只有 read、非 editor、非 wait、非 discovery 工具可以按 `max_parallel_tools` 成批并发。
- 默认并发度是 1。
- 批次用 `join_all`，结果按返回集合顺序追加。
- 写工具、编辑器、wait、discovery 仍串行。

`available_runtime_tools`（约 2796 行）的当前窗口：

- 普通 capability 初始窗口为 8。
- 受限 capability 初始窗口为 15。
- `read_image`、`web_search`、`web_fetch` 可能追加。
- pending execution 会被强制放回窗口。
- 总数超过 15 直接返回 `InvalidState`。
- discovered tools 按最新优先补到 15。

WP2 不改变工具批次的并发语义，只改工具可见性候选排序和模型观察结果。

## 3. 共享合同与不变量

### 3.1 三种结果不能混为一谈

实现时要严格区分：

| 结果 | 所有者 | 用途 | 是否可直接给模型 |
|---|---|---|---|
| Host 原始结果 | Host/domain command | 业务合同、重放、审计 | 否，可能过大或包含内部字段 |
| durable execution result | AgentApplication/DB | 幂等恢复、`result_refs`、对账 | 否，除非经过观察投影 |
| model observation | Agent runtime | 下一轮模型决策 | 是，必须有大小和错误边界 |

WP2 的 envelope 只约束第三行。不要把所有 Host 命令的返回类型改成一个跨领域 JSON envelope，那会把领域合同反向污染成 Agent 合同。

### 3.2 上下文 pack 不是权限

`stable_prefix` 只表达稳定性、缓存边界和压缩优先级，不表达权限。以下内容永远由 system message 或代码强制：

- scope 和资源归属。
- approval 要求。
- 禁止写入或 destructive 操作。
- lease、revision、editor revision。
- 不得把未成功的工具调用声称为成功。

模型可以看到这些规则，但删除它们不会放宽代码闸门。

### 3.3 回放必须使用原始快照

如果 model execution 已经 `succeeded`，必须反序列化该 execution 保存的 request/result，不重新计算当前工具窗口或上下文 pack。新 plan 只服务于新的 model execution。否则配置、工具目录或 Skill 变化会让历史 execution 发生非确定性重放。

### 3.4 未知副作用不可自动重试

`dispatching`/`running` 的非 read execution 如果没有 `safe` 或 `idempotent` replay policy，仍然转为 `outcome_unknown` 并等待 reconcile。WP2 的 `retryable` 字段不能覆盖这一条数据库级规则。

## 4. WP1：Agent 上下文工程接线

### 4.1 目标

WP1 完成后，每次新的 Agent model call 都应有：

1. 一个可回放的 `LlmContextManifest`。
2. 一个由 `ContextBudgetPlanner` 选出的 `LlmContextPlan`。
3. 一组稳定/可变 pack 引用及其内容 hash。
4. 一个可解释的 omission/dropped pack 记录。
5. 与实际发送消息一致的 `context_plan_json`、`context_pack_refs_json`、`context_manifest_json`。
6. 不改变已有审批、工具执行、消息持久化或模型协议行为。

### 4.2 不在 WP1 做的事

- 不自动检索项目、章节或蓝画正文。
- 不引入向量数据库、rerank、MQE、HyDE 或图检索。
- 不删除现有 `compact_context` 和 `fit_context_projection`，直到 shadow 结果证明可以安全收口。
- 不把红线从 system message 移到 user-content manifest。
- 不给 Skill 文本增加权限。
- 不对已完成的 model execution 重新规划上下文。

### 4.3 推荐的内部输入形状

不要求立刻暴露新的公共 crate contract；先在 `a4n-agent-runtime/src/engine/context.rs` 中建立 crate-private 输入结构：

```text
AgentContextInput
  model_context_tokens
  requested_output_tokens
  system_prompt                 // 仍作为 system message
  protocol
  tool_schemas
  paired_history_units           // assistant tool call + tool observations 保持原子
  pinned_user_messages
  thread_context_summary
  skill_directory_metadata
  current_task_content
  optional_memory_packets        // WP3 前为空
  optional_external_packets      // WP3/RAG 前为空
```

`tool_schemas` 参与预算，但不作为普通历史消息裁剪。工具窗口先由 WP2 选出，再交给 WP1 估算其 token 成本。

### 4.4 Pack 分类

第一版只组装已经存在的 Agent 输入，不主动增加领域事实：

| pack kind | 版本一来源 | required | stable prefix | 说明 |
|---|---|---:|---:|---|
| `agent_thread_summary` | `agent_thread.context_summary` | 否 | 否 | 只作事实参考，不能替代当前工具结果 |
| `agent_skill_directory` | `search_agent_skills` 的元数据 | 否 | 否 | 只放目录，不放所有 Skill 正文 |
| `agent_history_units` | 当前 run 的 paired history | 否 | 否 | 以完整 assistant/tool 批次为原子单位 |
| `agent_observation_refs` | execution 的摘要和 result refs | 否 | 否 | 大结果只保留引用，正文由工具按需取回 |
| `agent_plan_state` | WP4 未来的 plan/step | 否 | 否 | WP1 先保留 pack kind，不要求本批实现 plan |
| `agent_external_context` | 未来 memory/RAG | 否 | 否 | WP3 之前不自动填充 |

system prompt 仍由 `with_agent_system_prompt` 生成并保持 system role。若未来需要稳定的项目事实或输出契约，可以单独增加 stable pack；它不能取代 system 层的权限规则。

`dynamic_user_content` 只放当前任务或当前轮的用户输入。不要把全量历史复制到 dynamic user content，否则会失去 pack 的选择边界和 trace 可解释性。

### 4.5 计划流程

建议的调用顺序：

```text
1. pair_tool_messages(messages)
2. 生成 system_prompt（保留现有 with_agent_system_prompt）
3. 生成 LlmContextManifest
4. 为每个 pack 生成 full/focused/minimal variant
5. 用 ContextBudgetPlanner::plan_requirements_adaptive 选择 variant
6. hydrate selected pack content
7. render_context_manifest_user_content
8. 构造 [system_prompt, rendered_user_content] 加上协议所需历史/工具消息
9. 将 manifest/plan/pack refs 放入 ChatRequest 与 LlmTraceContext
10. 将最终 request snapshot 写入 model execution
```

这里的第 8 步必须根据 `ChatRequest` 和 provider protocol 的现有约束实现，不能简单把工具调用历史转成一段丢失 role 的文本。`pair_tool_messages` 负责 provider 合法性，manifest 负责上下文选择，两者不是互相替代。

### 4.6 Plan 序列化

写入 `LlmTraceContext.context_plan_json` 时使用 `serde_json::to_value(&plan)`，但需要确认 `selected_packs` 的 `serde(skip)` 行为不会把实际正文写入 trace。trace 只应包含：

```json
{
  "model_context_tokens": 32000,
  "output_token_reserve": 2048,
  "safety_margin_tokens": 512,
  "input_utilization_percent": 90,
  "raw_available_input_tokens": 29440,
  "available_input_tokens": 28928,
  "stable_prefix_tokens": 0,
  "dynamic_delta_tokens": 4380,
  "selected_compression_level": "focused_raw",
  "selected_pack_refs": [],
  "dropped_optional_pack_refs": [],
  "estimated_input_tokens": 4380
}
```

实际 `selected_pack_refs` 和 `dropped_optional_pack_refs` 由 planner 填充；示例中的空数组只是结构示意。

`context_pack_refs_json` 建议保存一个小型可查询投影，而不是重复整个 plan：

```json
{
  "selected": [
    {
      "pack_kind": "agent_history_units",
      "content_sha256": "...",
      "compression_level": "focused_raw",
      "token_estimate": 1800
    }
  ],
  "dropped_optional": [],
  "schema_version": 1
}
```

`ChatRequest.context_manifest_json` 保存 manifest 本身；provider trace 继续从 `LlmTraceContext` 取得 plan/ref。两个字段职责不同，不能只填其中一个。

### 4.7 历史兼容

现有 `history::fit_context_projection` 的重要行为必须保留：

- system message 总是保留。
- 当前请求和 pinned user message 保留。
- assistant tool call 与对应 tool result 不能拆开。
- 裁剪时插入明确的 omission warning。
- 缺失的 observation 不能被模型当成成功。

迁移方式：

1. 先把 `fit_context_projection` 的 message units 转成 `agent_history_units` 的 pack input。
2. WP1 shadow 阶段同时运行旧 projection 和新 planner，只发送旧消息，比较 token、保留/省略单元和结果 hash。
3. 通过 shadow 指标后，active 阶段使用 manifest renderer 的消息。
4. 保留旧 projection 作为 `minimal` 或故障兜底，不再让两套逻辑分别决定普通请求的最终历史。

现有 80% 容量触发的 `compact_context` 可以保留为 fallback：只有 planner 无法容纳 required pack，或历史摘要水位落后时才调用。不能形成「先旧 projection 压一次，再 manifest 压一次，再 summary 再压一次」的三重压缩链。

### 4.8 稳定前缀与安全

`stable_prefix` 的第一用途是让上下文 renderer 和 provider cache 知道哪些 pack 在本次请求前缀中稳定。它不等于 system message，也不等于权限。

安全规则：

- 权限、审批和资源范围继续放在 system prompt + 代码闸门。
- 用户的「不要做某事」如果属于当前请求的安全红线，放入当前 Agent system policy 的 pinned section，而不是普通动态历史。
- 摘要/压缩前提取安全约束只是辅助；不能用模型摘要替代代码层确认。
- 大工具结果只做引用卸载，不在压缩时修改 execution 的完整结果。

### 4.9 trace 与 analytics 接线

需要修改的最小位置：

| 文件 | 设计改动 |
|---|---|
| `crates/a4n-agent-runtime/src/engine/mod.rs` | 调 context builder；把 plan/refs/manifest 填入 request；保留 replay snapshot |
| `crates/a4n-agent-runtime/src/engine/context.rs` | 新增 crate-private pack 构造、planner 调用和 manifest 渲染适配 |
| `crates/a4n-llm-context/src/manifest.rs` 或 `lib.rs` | 如需 cache boundary，公开一个窄的 `(content, boundaries)` helper；不复制 renderer |
| `crates/a4n-runtime/src/dynamic_llm/trace/recording.rs` | 不改逻辑，确认现有透传覆盖新增字段 |
| `crates/a4n-db/src/admin/generation_analytics/*` | 预计不改；已有 metrics 读取字段，补测试即可 |

如果仅填已有 trace 字段就能完成第一版，不要新增数据库列或 Agent 专用 trace 表。

### 4.10 WP1 验收

必须有以下最小测试：

1. `LlmContextPlan` 序列化不包含 pack 正文，包含 selected/dropped refs 和 token metrics。
2. system prompt、当前用户消息、完整 tool call/result 单元在 planner 压力下仍满足保留规则。
3. 大 observation 仍只给模型 stub，完整结果仍能通过 `read_agent_execution_result` 读取。
4. shadow 模式的旧 projection 与新 plan 记录同一批 history watermark，不产生隐式额外 model call。
5. model execution replay 使用持久 request snapshot，不重新读取当前工具窗口。
6. `workflow = "agent"` 的 `llm_call_trace` 能出现 `context_plan_json` 和 `context_pack_refs_json`。
7. `generation_analytics` 能从 Agent trace 投影 stable prefix、dynamic delta、estimated input 和 dropped optional 数量。
8. provider 不可用、上下文过大、manifest 校验失败时，错误进入既有 failure/recovery 路径，不自动执行写工具。

推荐验证命令（代码实施时）：

```text
cargo test --locked --offline -p a4n-llm-context
cargo test --locked --offline -p a4n-agent-runtime
cargo test --locked --offline -p a4n-db --lib admin::generation_analytics
cargo fmt --all -- --check
git diff --check
```

真实 PostgreSQL/worker E2E 仍需按 `docs/TEST_DATABASE.md` 使用独立数据库；未配置数据库时不能把跳过的集成测试报告为通过。

## 5. WP2：工具契约、错误诊断与窗口治理

### 5.1 目标

WP2 完成后，模型每次收到工具观察都能回答三个问题：

1. 工具是否成功、部分成功还是失败？
2. 失败的具体原因是什么，是否允许修正后重试？
3. 下一步应该重新读取、改参数、等待对账、请求审批，还是向用户报告？

同时，工具 schema 窗口超过 15 时应裁掉可选工具并记录原因，而不是因为可选工具数量超过上限直接把 Run 变成 `InvalidState`。

### 5.2 不在 WP2 做的事

- 不改变 `AgentRuntimeHost` 的 Host command 参数和返回合同。
- 不把所有领域工具重写成一个通用 Tool trait。
- 不改变 read-only 工具的并发规则。
- 不对未知副作用自动重试。
- 不把错误改成只对模型友好的自然语言；完整错误仍落 execution/event。
- 不把 `summary` 的改写当作权限控制。

### 5.3 模型观察 envelope

建议在 runtime 内部定义一个 crate-private `AgentToolObservation`，在 `append_tool_observation` 边界生成。第一版 wire 形状：

```json
{
  "observation_version": 1,
  "status": "success | partial | error",
  "data": {},
  "text": "给模型的短说明",
  "error": {
    "code": "CONFLICT",
    "retryable": true,
    "next_action": "read_before_edit",
    "details": {}
  },
  "result_refs": [],
  "execution": {
    "id": "123",
    "complete": true,
    "read_tool": null
  },
  "stats": {
    "duration_ms": 120,
    "result_bytes": 512
  }
}
```

字段规则：

| 字段 | 规则 |
|---|---|
| `status` | 只允许 `success`、`partial`、`error` |
| `data` | 小型、与领域结果同构的可用数据；过大时为空或只含 preview |
| `text` | 低 token、可读的事实说明；不能是唯一错误载体 |
| `error` | `status=error` 时必填；包含稳定 code、是否可修正、下一动作 |
| `result_refs` | 复用现有资源引用语义；去重按 kind/id |
| `execution` | 让模型知道是否可用 `read_agent_execution_result` 继续取回 |
| `stats` | 只放诊断所需的稳定数值，不放 secret 或内部栈 |

`agent_execution.result_json` 仍保存 Host 原始结果；envelope 只进入 tool message 和模型观察。这样 `discovered_tool_names`、`tool_result_refs`、领域 Host 仍可以解析原始 durable result，不需要一次性迁移所有工具。

### 5.4 状态映射

在 `append_tool_observation` 前统一按 execution 和 Host 结果生成 envelope：

| execution/Host 情况 | observation | 模型可采取的动作 |
|---|---|---|
| read/write 成功且结果完整 | `success` | 使用数据或继续任务 |
| 结果完整但超过观察上限 | `partial` + execution ref | 调 `read_agent_execution_result` 精确分页 |
| 参数错误、资源不存在、查询无结果 | `error` + `retryable=true` 或 `false` | 修正参数；真正无结果时停止重试 |
| revision/mtime/editor conflict | `error` + `code=CONFLICT` | 先重新 inspect/read，再 edit |
| 权限/scope/approval 拒绝 | `error` + `retryable=false` | 不重复调用，向用户说明或等待审批 |
| Host 临时不可用 | `error` + `code=EXTERNAL_UNAVAILABLE` | 可在预算内改变等待/报告策略 |
| 已派发但无回执 | 不生成 success observation | Run 等待 `reconcile`，禁止重复写入 |
| 协议解码或 schema 错误 | `error` + `code=PROTOCOL_ERROR` | 可触发一次修正决策，不能无限自动重试 |

特别是「确实没有匹配结果」和「查询路径错误」必须不同：

```json
{
  "status": "success",
  "data": {"items": []},
  "text": "查询成功，但没有匹配项"
}
```

与：

```json
{
  "status": "error",
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "retryable": true,
    "next_action": "check_scope_or_identifier"
  }
}
```

不能把两者都压成空字符串，否则模型只能赌运气换参数。

### 5.5 错误 code 与重试规则

第一版只定义少量稳定 code，不建立巨大的错误字典：

```text
INVALID_ARGUMENT
RESOURCE_NOT_FOUND
CONFLICT
FORBIDDEN
APPROVAL_REQUIRED
EXTERNAL_UNAVAILABLE
TIMEOUT
PROTOCOL_ERROR
OUTCOME_UNKNOWN
RESULT_TRUNCATED
```

`retryable` 只表达「改正输入或等待后是否有意义」，不表达「系统可以自动重试」。

运行时应保存当前 Run 内的调用 fingerprint：

```text
fingerprint = sha256(tool_name + canonical_json(arguments) + error.code)
```

同一个 fingerprint 连续出现 3 次时，生成 `TOOL_RETRY_LIMIT` 观察并停止同参数重试。第一版不做跨 Run 的全局 circuit breaker；如果以后要处理供应商持续故障，应在 Host/sidecar owner 增加按供应商维度的熔断，而不是把状态塞进 Agent history。

### 5.6 工具描述质量

现有 `AgentToolDescriptor`（`a4n-api-contract/src/agent.rs:1097-1111`）已经有：

- `name`
- `summary`
- `domain`
- `effect`
- `exposure`
- `risk`
- `reversibility`
- `replay_policy`
- `cost_class`
- `tags`
- `args_schema`
- `result_schema`

WP2 不增加第二份 descriptor。优先改进 canonical command descriptor 生成的 `summary`，让同一描述同时服务 discovery、schema 和运行审计。

每个模型可见工具的 `summary` 至少回答：

1. 什么时候使用。
2. 什么时候不要使用。
3. 输入 ID/版本/范围是什么。
4. 返回的是事实、建议、草稿还是已提交副作用。
5. 失败后应该先做什么。

示例结构：

```text
读取当前章节的结构化上下文。只用于确认事实或定位资源，不会修改章节。
必须提供 projectId/chapterId；返回分页事实和 resource refs。
若返回 CONFLICT 或版本不一致，先重新读取，不要使用原参数重复写入。
```

描述不应承担权限声明。即使模型看不到 `summary` 的最后一句，代码仍必须在 capability、scope、phase 和 approval 层拒绝越权。

### 5.7 工具窗口新算法

窗口 builder 仍位于 `available_runtime_tools`，但从「先拼接、超限报错」改为「候选分类、过滤、按优先级裁剪」。

#### 5.7.1 候选过滤

按顺序执行：

1. 从 canonical `agent_execution_tool_descriptor` 取 descriptor。
2. 过滤 capability 不允许的工具。
3. 过滤 scope 不允许的工具。
4. 过滤当前 phase 不允许的工具。
5. 去重并保留 canonical name。
6. 计算 descriptor/schema token 成本。

任何模型返回的 discovered name 都必须重新走 `tool_descriptor`，不能信任模型返回的 schema；当前 debug 已有这一条保护，保留。

#### 5.7.2 pending execution 处理

pending tool batch 的恢复已经在 model call 前执行（`process_claimed_run_inner` 约 953-983 行）。因此 pending execution 不应无限挤占新的 schema 窗口：

1. 先恢复 pending batch。
2. 若执行完成，新的 model call 只构建当前可用工具窗口。
3. 若仍在 approval/external/reconcile 等 waiting 状态，直接释放 Run，不发新的 model request。
4. 若发现 pending 状态与恢复结果不一致，进入 `recovery_required` 或 `reconcile`，而不是为了凑满 schema 返回 `InvalidState`。

这样既不隐藏未完成副作用，也不需要把任意数量的 pending tool schema 塞进一次请求。

#### 5.7.3 优先级

对最多 15 个 schema 的软窗口，建议顺序：

| 优先级 | 候选 | 说明 |
|---:|---|---|
| 1 | 当前 capability 的必要 base tools | 保证最小闭环 |
| 2 | 当前 phase 的必需工具 | discover/context_read/apply/verify 等按 phase 变化 |
| 3 | Skill 工具 | `search_agent_skills`/`load_agent_skill`，它们不授予权限 |
| 4 | 最近成功 discovery 的工具 | 新发现优先，仍经过 canonical descriptor |
| 5 | 通用只读工具 | `read_image`、`web_search`、`web_fetch` 等按 capability/scope 过滤 |

受限 capability 仍只暴露它的初始操作目录，不自动追加 discovered tools。main/general 保留 discovery room，但 optional 工具被裁掉时要写入 context plan 的 dropped refs 或 tool-window diagnostic。

#### 5.7.4 超限策略

- optional 工具超限：按优先级裁掉，不报 `InvalidState`。
- 一个 mandatory 工具的 schema 本身超过可用输入预算：返回明确的 `TOOL_SCHEMA_TOO_LARGE`，Run 等待/失败原因必须可见；不能静默截断 JSON schema。
- mandatory 工具数量超过 15：这是 capability manifest 的配置错误，不由模型运行时吞掉；保留显式失败，但错误必须指出 capability、工具名和配置来源。
- schema 摘要截断不允许破坏 JSON Schema；需要缩减时改 descriptor 文案、args description 或工具集合，而不是字符串裁剪 JSON。

### 5.8 并发边界保持不变

WP2 不把「schema 并发可见」和「工具执行并发」混在一起。继续保持：

- read + 非 editor + 非 wait + 非 discovery：可按 `max_parallel_tools` 批次执行。
- editor、wait、discovery、所有写工具：串行。
- 写工具进入 Host 前仍先过 approval。
- `join_all` 的结果顺序和 execution id 不能被 envelope 改变。

若以后要修改并发，必须另开设计与测试批次，因为它会影响副作用顺序、预算预留和 durable execution replay。

### 5.9 WP2 文件边界

| 文件 | 设计改动 |
|---|---|
| `crates/a4n-agent-runtime/src/engine/mod.rs` | observation envelope、error mapping、fingerprint guard、window candidate priority |
| `crates/a4n-agent-runtime/src/engine/history.rs` | 将 bounded result 投影成 `partial` observation；保留完整 execution result |
| `crates/a4n-api-contract/src/agent.rs` | 只在确有需要时补稳定 error/observation schema；不复制 descriptor |
| `crates/a4n-agent-application/src/agent.rs` | 预计不改 Host/domain 返回合同；若要持久化 observation metadata，只加窄字段或 payload version |
| `crates/a4n-control` / Host adapters | 预计不改；错误由 runtime observation boundary 归一化 |

第一版可以不加数据库迁移：完整原始结果和 `error_message` 已有持久化位置，模型 observation 是 message payload。若要让 UI 逐字段展示 error code，再单独评估是否需要 `agent_event` projection，不要为了 WP2 新建表。

### 5.10 WP2 验收

1. `success`、`partial`、`error` 三种 observation 都能反序列化，并拒绝未知 `observation_version`。
2. 大结果保留完整 execution，模型只收到带 `execution.id` 的 partial stub。
3. `RESOURCE_NOT_FOUND`、空结果、`CONFLICT`、`OUTCOME_UNKNOWN` 在模型侧表现不同。
4. 同一 tool/args/error fingerprint 连续三次后停止同参数重试。
5. `outcome_unknown` 不会被 envelope 包成 success，也不会触发第二次写入。
6. discovery 的 durable raw result 仍能恢复工具名，model observation 改成 envelope 不影响发现缓存。
7. director 或其他 32 项工具目录在可选工具过多时不再因总数超过 15 直接失败；裁剪原因可追踪。
8. restricted capability 仍只收到其允许的初始工具；scope/phase/approval 规则不回退。
9. read-only 可并行批次、写/editor/wait/discovery 串行的现有测试继续通过。
10. 工具 summary 能说明使用场景、输入资源、结果语义和失败后的下一动作；descriptor schema 没有被字符串截断。

推荐验证命令（代码实施时）：

```text
cargo test --locked --offline -p a4n-api-contract agent
cargo test --locked --offline -p a4n-agent-runtime
cargo test --locked --offline -p a4n-agent-application
cargo fmt --all -- --check
git diff --check
```

## 6. 实施顺序与提交拆分

建议拆成四个小提交，不把 WP1/WP2 混成一次大改：

```text
1. fix: repair debug Agent generation owner precondition join
2. test/refactor: add Agent context shadow plan and trace projection
3. feat: enable Agent context manifest after shadow assertions
4. feat: add Agent observation envelope and soft tool-window policy
```

### 6.1 Shadow 阶段

- 新 planner 只计算，不改变发送给模型的 messages。
- 记录 legacy projection 与 new plan 的 token/pack/dropped 对比。
- 检查同一 `input_message_id`、history watermark 和 tool batch 边界。
- 只允许 read-only capability 先进入 active 阶段。

### 6.2 Active 阶段

- model request 使用 manifest 生成的 user context。
- 每个新 model execution 保存 request snapshot。
- trace analytics 用 Agent workflow 的实际数据观察上下文预算。
- 发现关键事实丢失、工具批次破碎或 token 成本上升时，退回 legacy projection；不要删除旧实现直到有足够轨迹。

### 6.3 回滚边界

| 问题 | 回滚方式 |
|---|---|
| context plan 计算错误 | 关闭 active strategy，继续 legacy projection |
| manifest renderer 错误 | 从 request snapshot 重放旧 execution，不重跑副作用 |
| observation envelope 解析错误 | 新消息兼容 raw legacy observation；不回写历史消息 |
| tool window 裁剪错误 | 恢复旧窗口函数，但保留诊断事件；不放宽代码权限 |
| Host 返回合同变化 | 回滚 runtime observation adapter，不改 Host/domain 合同 |

历史 tool message 不做批量回填。新代码必须能读取 legacy raw observation；新 envelope 通过 `observation_version` 区分。

## 7. 参考 Hello-Agents 的部分与不照搬的部分

### 7.1 采用

- ReAct 的观察闭环，但把错误分类成结构化数据。
- LangGraph 的显式 state 思路，但把 state 放进 A4N 已有 durable turn/run/execution，而不是进程内 TypedDict。
- 第九章的 GSSC：先收集候选、再选择、固定结构、最后压缩。
- Extra09 的结论：可诊断性是可恢复性的前提；保留调用链、返回链和决策链。
- Extra08 的渐进式披露：目录轻、正文按需、确定性操作交给代码。

### 7.2 不采用

- 不照搬 Plan-and-Solve 的局部变量 plan；后续 plan 必须持久化。
- 不照搬 Reflection 的字符串终止词；未来 verify 必须有结构化验收和预算限制。
- 不照搬 Python 的 `MemoryTool.execute(action, **kwargs)` 字符串分发；Rust 侧保持强类型输入。
- 不把 MCP/A2A/ANP 作为内部架构层。现有 manifest、Host contract、worker protocol 已覆盖产品内部边界。
- 不做运行时自动安装第三方 Skill/MCP；供应链、权限、版本和回滚尚未具备时只能做受审资产。

## 8. 后续工作边界

WP1/WP2 完成后，仍有三个独立问题，不应偷偷塞进本批：

1. **记忆管线**：当前 `agent_memory` 只有 scope、revision、status、content 和 resource refs；`read_memory_for_run` 仍按 key/content 做大小写包含匹配和 revision 去重。importance、recency、embedding/RAG、遗忘策略应另开 WP3。
2. **plan/verify**：应把 plan 放进 turn/run 的 durable state，并把 verify 作为只读证据步骤；不是在 WP1 里把任意计划文本塞进上下文。
3. **评估闭环**：复用已有 `llm_call_trace`、`agent_execution`、`agent_event`，另建离线 5→50→全量门禁；不要在 WP1/WP2 热路径调用常态 LLM judge。

## 9. 参考资料

- [Hello-Agents 在线文档](https://hello-agents.datawhale.cc/#/)
- [Hello-Agents GitHub](https://github.com/datawhalechina/hello-agents)
- [Hello-Agents 第九章：上下文工程](https://github.com/datawhalechina/hello-agents/blob/main/docs/chapter9/%E7%AC%AC%E4%B9%9D%E7%AB%A0%20%E4%B8%8A%E4%B8%8B%E6%96%87%E5%B7%A5%E7%A8%8B.md)
- [Hello-Agents 第十二章：性能评估](https://github.com/datawhalechina/hello-agents/blob/main/docs/chapter12/%E7%AC%AC%E5%8D%81%E4%BA%8C%E7%AB%A0%20%E6%99%BA%E8%83%BD%E4%BD%93%E6%80%A7%E8%83%BD%E8%AF%84%E4%BC%B0.md)
- [Hello-Agents Extra09：Agent 应用开发踩坑](https://github.com/datawhalechina/hello-agents/blob/main/Extra-Chapter/Extra09-Agent%E5%BA%94%E7%94%A8%E5%BC%80%E5%8F%91%E5%AE%9E%E8%B7%B5%E8%B8%A9%E5%9D%91%E4%B8%8E%E7%BB%8F%E9%AA%8C%E5%88%86%E4%BA%AB.md)
