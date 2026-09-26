# 模型输入优化第二轮收尾

本轮已完成，直接修改 `debug`，沿用用户授权。相对 `data/prompt-context-audit-20260926/cleanup/before/`，76 个代码、测试和模板文件新增 676 行、删除 900 行，净减少 224 行；其他会话 WIP 未计入本轮成果。改动保留为未提交状态，未整体提交、合并或推送。

## 已修复与删除

- 蓝图阶段的三份相同变体合为一份；蓝图精修、分部和章节大纲删除会丢失人物、关系、世界设定、关键事件和用户中段要求的降级投影。保留输入结构校验，无蓝图时使用正式项目资料的业务能力继续有效。
- 大纲的蓝图来源包归并到同一个构造点，要求明确 `blueprint_available`，删除默认为有蓝图的旧行为。蓝图精修将稳定的灵感历史放在不断修改的蓝图之前。
- 模型用灵感历史取消最近 20/80 条的隐式截取；界面历史列表仍限 80 条。按消息 ID 确定对话顺序，删除为排序而伪造助手时间戳的 1 毫秒偏移。
- 共用 JSON 请求序列化改为紧凑格式，保留稳定根字段顺序、字符串内容和输入标记；蓝图阶段、精修和视频调用同步使用紧凑输入。
- 视频语言说明只发送一次，单帧、API 视频和 Chat 各自只发送需要的章节。生成提示与校验共用中文/基础长度下限：视频 visual 100、motion 60、frame 80。英文仍需满足原有词数或字符密度，并同时满足 Provider 最低字符数，消除“生成通过、提交被拒”的边界漏洞。
- 删除视频账本的 `raw_output`、`placeholder`、`asset_context`、模型输入中无用的上一阶段包装、重复构建流程及 11 个无调用规则。只记录当前阶段和公共 system 模板依赖；已进入动态请求的选中规则参与现有请求指纹，不再额外绑定整份其他阶段规则。
- 删除可绕过逐镜头模板/来源校验的聚合断点复用及其辅助函数、输入字段、序列化和指纹。`video_segment_plan.final` 仅保留为冻结草稿编辑的完成标记；恢复经过已有逐镜头验证，草稿执行方案仍保存在原有镜头产物中。
- 带标记与纯文本来源裁剪共用字符范围、原文匹配和重叠检查；支持 LF、CRLF、CR，避免 CR 文本把不相关段落带入请求。
- 删除 Agent 中会被 Runtime 缓存配置覆盖的两处 key 设置，保留 Runtime 按项目和模型配置缓存的单一入口。
- 测试工具按 task 名识别请求，消除对 JSON 空格格式的依赖；灵感路由只读取 system 身份，避免蓝图精修的历史消息误触发灵感 fixture。任务等待使用实际返回的 attempt，删除默认为 1 的错误假设。过时测试改为校验 JSON 结构和当前合同。

## 关键入口

| 规则 | 文件 |
| --- | --- |
| 紧凑 JSON 与稳定字段顺序 | `crates/prompt/a4n-prompt/src/request.rs` |
| 灵感完整历史与消息顺序 | `crates/platform/a4n-db/src/inspiration/conversations.rs` |
| 精修来源完整性与缓存顺序 | `crates/prompt/a4n-prompt/src/blueprint_generation/refine/context.rs` |
| 大纲共享来源包 | `crates/host/a4n-orchestrator/src/outline/context.rs` |
| 视频语言与长度合同 | `crates/host/a4n-orchestrator/src/video/prompts/json/validation/prompt_text/density.rs` |
| 逐镜头恢复与最终冻结 | `crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/staged_segment_plan.rs` |
| 原文校验和段落裁剪 | `crates/platform/a4n-domain/src/source_spans.rs` |

## 验证

当前源码 514 项定向 Rust 测试、29 项 Mock Provider 测试、28 项 CLI runner 测试通过。完整模式 Server/CLI/Worker/迁移程序构建、全仓 rustfmt 和 `git diff --check` 通过。Rust 覆盖包括 PostgreSQL 实际读取、草稿冻结保护、模板变更指纹、源文本完整性及长历史边界。

实际 CLI 验证通过 55 个文本场景、17 项导演/媒体检查及 8 项四协议缓存检查。41 轮灵感对话始终保留首轮约束；39 次相邻请求完整延续已有消息前缀，两次蓝图精修均包含全部 82 条历史，且首轮约束只出现一次。精修请求共享的灵感前缀为 24,538 UTF-8 字节。视频恢复保留手工片段、执行方案和未受影响的视频，图片 negative 到达 Provider 一次。

结果见 [cleanup-validation.json](cleanup-validation.json)，请求证据见 [cleanup-wire-evidence.json](cleanup-wire-evidence.json)。完整日志、固定二进制校验值和本轮补丁在 `data/prompt-context-audit-20260926/cleanup/`。CLI 覆盖的是上述指定业务路径，419 个命令的完整 manifest 未全部执行。

首轮 PostgreSQL 容器重启后端口从 32768 改为 32770，旧连接导致测试超时；修正本轮私有环境后重跑通过。该失败记录保存在 `cleanup/first-attempt-*` 和 `cleanup/first-attempt/`，不计为通过。四个旧断言依赖缩进或已废止的 80 字下限，修正后复验通过，原始记录在 `cleanup/before-test-assertion-update/`。

首次完整链接缺少 ONNX 链接变量，改用项目已有 `require_onnxruntime` 后通过，原始诊断保存在 `onnx-link-failed-e2e-build.*`。长对话暴露的 runner attempt 错误及临时账本绝对文件路径错误均已修正，失败记录保留；文件路径边界校验未放宽。

临时 API、Worker、Mock、Qdrant 和 PostgreSQL 均已停止，样本与数据库保留。共享构建目录在持锁且固定二进制后清理，未终止其他会话进程。

## 实际行为边界

- 必要上下文超过模型预算时明确报错，不再靠截取历史或删掉事实装作成功；本轮没有增加摘要模型调用。
- 模板与上下文变化会影响断点指纹。保留用户已发布内容和执行方案的业务复用；生成断点必须通过当前来源和合同校验。
- Mock 用于核实实际输入、调用次数和缓存协议。没有把合成 usage 当成真实命中率，也没有宣称已完成远端模型质量或缓存率 A/B。
