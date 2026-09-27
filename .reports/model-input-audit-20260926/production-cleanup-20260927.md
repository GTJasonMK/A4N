# 生产调用链再审与收尾（本轮完成）

延续用户直接修改 `debug` 的授权。本轮从上一轮测试清理继续审查生产代码，搜索全仓兼容、回退、弃用、吞错和死代码标记，并追踪模型请求、上下文读取、权限、持久化和调用者。未把其他会话的拍摄合同、视频、ORM 或 UI 改动计入成果。

## 已修正

| 问题 | 根因修复 |
| --- | --- |
| 优化对话丢失旧要求和长回复 | 删除 12 轮和单条 6,000 字截断；仅传递完整用户文本、回复和终态，移除截断标签、字符计数及时间戳。模型预算仍由现有上下文规划器校验 |
| 每步重复发送历史工具参数 | 原始观察已经逐条追加，但 `progress` 又带完整参数、读操作和补丁结果；删掉第二份投影，只保留观察计数，模型仍能读取全部原始观察 |
| 同一协议多处重复 | 优化聊天、分析和补丁的 user payload 删除已由 system 定义的动作、schema、权限说明和完成规则；共用循环规则只承担停止策略，动态配置和用户指令移到固定 system 规则之后 |
| 正文与原文引用可能来自两次查询 | 在共享章节投影中同时保存正文和引用，删除后续再次读取/覆盖正文的路径；补丁模型装配明确移除重复正文和分句，完整正文只发送一次 |
| 段落正文与引用错配 | 删除 Agent 私有的“空行分段”实现；Domain 共用同一段落范围与引用索引，支持 LF、CRLF、CR 和开头空行，引用身份和原文字节范围保留 |
| 搜索结果不能直接读取 | 两套搜索合并；返回标准 JSON Pointer，正确处理数组、斜杠、波浪号和点号；修复 Unicode 小写展开造成的摘要位置偏移 |
| 权限损坏时回退全读 | 读取和补丁共用最新未取消会话的选择；存在会话时使用原有严格权限解析，不再跳过无效记录或默认授予所有读取能力；无会话时的初始策略共用一处定义 |
| 关闭大纲读取仍返回当前章纲 | 数据库上下文边界统一遵守 `read_outlines`，同时校验章节属于声明项目，错误项目不能读取章节或相关资料 |
| 未实现参数和隐式兼容 | 删除从未生效的搜索 `cursor`；不支持分页的读取/目录请求显式拒绝 cursor；上下文来源名称直接来自同一常量目录，删除重复枚举表 |
| Mock 仍识别旧提示词和旧观察数组 | 使用真实基础任务 JSON 与原始观察序列；删除旧 Markdown 指令解析、未使用的等待分支和旧 observations 回退；用现有 CLI 流程复验 |

关键文件：

- `crates/host/a4n-orchestrator/src/optimize/messages/`、`optimize/agent_loop/request/step.rs`：模型最小输入与完整历史。
- `crates/host/a4n-orchestrator/src/optimize/runtime_tools/`：搜索、读取、权限、原文引用。
- `crates/platform/a4n-domain/src/source_spans.rs`、`crates/platform/a4n-db/src/optimize/context.rs`：共享段落与数据边界。
- `crates/host/a4n-api-contract/src/optimize.rs`、`frontend/src/api/generated/host.ts`：真实生成器同步的输入合同。
- `scripts/tests/run_agent_domains_cli_e2e.py`：搜索→读取→引用和 14 轮长历史验收。

## 保留项及依据

历史请求快照中的旧缓存字段转换、旧编辑器来源身份恢复、拍摄合同/项目包迁移仍处理真实持久化数据；不能仅凭 `legacy` 或 `compat` 名称判为冗余。恢复仍须通过当前来源、权限和数据校验。平台默认值、可选预取失败和日志自身的失败处理也未当作业务降级删除。导出、视频、RAG、Native 和其他领域的独有测试继续保留。

## 验证账本

- Python：157 项通过，`ResourceWarning` 按错误处理。
- 前端：当前工作区 429 项通过。
- Rust：`second/` 中 Domain 原文 10、API 1、Orchestrator 优化 60、LLM Context 24、真实 PostgreSQL 查询 2 项，共 97 项通过。包含无效/缺失权限、取消会话、跨项目拒绝、关闭大纲读取、完整历史、单份正文与工具参数。
- 第一次源码快照捕获并行视频代码的中间状态：finalize 仍传递已移除字段、两个测试缺少 `Some` 包装；失败原样保留。对应会话随后修正了根工作区，本轮未重复修改这些文件，第二次编译通过。
- Server、CLI、迁移和 Worker 的最终构建通过，固定程序和源码位于 `second/`。
- 七组 CLI 共 84 项通过（包含资料回写组的独立复核）；完整九阶段生产浏览器流程通过。Agent 用例实际执行搜索→按路径读取→段落引用核对，并跑完 14 轮优化对话，每轮直接答复只有一次模型调用。
- 实际请求保留 6,559 字首轮要求和 6,526 字首轮回复；前者在所有请求中仅一份，后者在后续请求中仅一份。已有历史的相邻请求共享 43,906–46,373 字节消息前缀，详见 `wire-context-evidence.json`；这是请求字节证据，不是真实缓存命中率。
- API/生成合同检查在共享构建锁内通过；首次等待锁超时退出 75 不计通过，后续复核为 0。核心边界、本任务 Rust 格式及 `git diff --check` 通过。
- 全仓 rustfmt 最后仍在并行修改的 `context/workflow/timing_upgrade.rs` 和 `video/prompts/generation/frame_prompts/staged_segment_plan/shots.rs` 报差异，未代为整理这些文件，也未把全仓门禁记作通过。

## 原始失败、版本与资源

第一次端到端准备在并行 Galgame 类型变更处失败：冻结前端把可空 outcome 传给字符串标签函数。根工作区已由相应会话改为读取规范 status；验证快照只同步这两行现有修正，记录于 `web-validation/overlay.json`，后端保持原构建版本。后续正式验收先通过六组 CLI 和九阶段浏览器，资料回写组仍断言旧包版本 v6；按正式 `PROJECT_FAMILY_PACKAGE_VERSION` 更新为 v7 后，在新隔离环境完成全部 8 项往返、事实和身份检查。

机器账本 `data/production-cleanup-20260927/validation-final.json` 检查了两次验收的应用/前端指纹一致、七组成功结果及九阶段浏览器结果。原扩展报告仍保留失败状态，账本明确标为复核后的汇总；未改写旧报告制造一次全绿。测试之后本任务仅有一处函数调用换行的 rustfmt 差异，其他并行工作区改动不计入验收。

所有本轮 Server、Worker、Mock、Qdrant、浏览器和 PostgreSQL 容器已关闭，证据 `resources-final.json`；共享 target 已在锁内清理，未终止其他会话服务。改动直接保留在 `debug`，未新建分支、整体提交、合并或推送；协作记录继续保留。

原始证据保存在 `data/production-cleanup-20260927/`；CLI/浏览器报告位于 `data/creation-e2e/production-cleanup-20260927-{second,facts}/`。通过结果只证明记录版本和明确测试场景；模型使用本地 Mock，不是远程模型质量或真实缓存命中率的 A/B，也不代表整个并行修改中的 `debug` 已没有问题。
