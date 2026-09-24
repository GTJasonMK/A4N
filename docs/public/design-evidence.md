# 设计依据 / Design evidence

[中文首页](../../README.md) · [English README](../../README.en.md) · [实际演示与验证](demo.md)

以下机制依据应用基线 `e2f3aa0dcdd7a0ab613e986050eae568a9850c56` 的实现梳理。代码链接用于定位规则；本次实际执行的验证范围单独记录在 [validation.json](validation.json)。

| 值得关注的设计 | 对创作的意义 | 实现依据与适用范围 |
| --- | --- | --- |
| **关键提取回指原文** | 事实、实体与伏笔有可核对的出处 | [片段引用校验](../../crates/host/a4n-orchestrator/src/chapter_analysis/owner/materialize/values/source_spans.rs)要求引用非空且能在本章原文片段中解析；[实体与事件落地](../../crates/host/a4n-orchestrator/src/chapter_analysis/owner/materialize/pipeline.rs)复用这一规则。引用成立仍需作者判断提取含义。 |
| **分析后继续沉淀资料** | 一次章节分析继续更新后续创作需要的资料 | [分析后的同步流程](../../crates/host/a4n-orchestrator/src/chapter_analysis/application/analysis.rs)处理回顾章纲、分析/伏笔/RAG、物体发现、人物发现、场景和人物资料；[章纲来源规则](../../crates/platform/a4n-db/src/project/outline_records/provenance.rs)保留作者正式章纲。分部大纲与蓝图不由此自动改写。 |
| **围绕当前写作任务检索** | 章纲、人物与未解线索会影响本次取回哪些前文 | [查询构造](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/query.rs)从章纲、写作要求、角色、待回收伏笔和场景生成候选查询；存在数量上限，按实际资料取用。[执行](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/execution.rs)调用混合检索并合并结果。 |
| **检索遵守章节时间** | 写第 n 章时，前文检索限定为第 1 至 n−1 章 | [检索过滤](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/preparation.rs)与[范围规则](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/rules.rs)同时约束章节范围和 Embedding 身份。这是生成前序检索的边界，不代表所有其他上下文来源都受同一个过滤器处理。 |
| **连续性有专门的上下文位置** | 人物状态、事件、摘要与原文按用途组织 | [检索融合](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/fusion.rs)考虑章节距离；[上下文组装](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context/formatting/context.rs)按关键连续性、摘要和片段分配预算，转移未用余量；[生成变量](../../crates/host/a4n-orchestrator/src/chapter_generation/preparation/variables.rs)将结果放入写作请求。 |
| **局部补丁与作者确认** | 修改可先预览，原文变化后旧提案不能直接覆盖 | [上下文读取](../../crates/host/a4n-orchestrator/src/optimize/runtime_tools/agent_context.rs)提供可选段落与来源哈希；[补丁应用](../../crates/host/a4n-orchestrator/src/optimize/patch/apply.rs)处理写回；[实际预览](../../frontend/src/components/agent/ContentDiffPreview.tsx)提供应用与拒绝。演示实际验证了确认前不写回及确认后仅应用所审修改。 |
| **派生作品保留来源版本** | 原稿更新后可定位章节与档案变化 | [章节差异](../../crates/platform/a4n-db/src/source_sync/snapshots/chapter_diff.rs)比较正文与分析哈希、增删和次序；[档案差异](../../crates/platform/a4n-db/src/source_sync/snapshots/profile_diff.rs)比较档案内容；[决策保护](../../crates/platform/a4n-db/src/source_sync/decisions/guards.rs)对已有本地修改要求相应处理选择。自由视频画布是独立工作流，不因此获得小说来源快照。 |
| **视频帧与片段共用规划** | 不同制作阶段可以围绕同一个片段继续工作 | [共享片段规划](../../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/frame_chain/segment_plan.rs)保存片段、执行配方与来源身份；[帧链编排](../../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/frame_chain/orchestration.rs)按片段读取该规划；[请求约束](../../crates/host/a4n-orchestrator/src/video/provider/request_rules.rs)检查首帧链所需的前段尾帧。它约束输入关联，生成画面仍须审查。 |
| **按输入身份复用完成阶段** | 长流程的恢复可以保留已完成工作 | [LLM 检查点](../../crates/platform/a4n-db/src/workflow/llm_checkpoint.rs)按输入指纹读取完成结果；[分析阶段恢复](../../crates/host/a4n-orchestrator/src/chapter_analysis/stages/runner.rs)先检查可复用结果；[生成源校验](../../crates/host/a4n-orchestrator/src/chapter_generation/service/generation/checkpoint/fingerprint.rs)阻止使用已经变化的正式资料。是否可恢复取决于具体工作流和当前状态。 |
| **多端共用命令与业务边界** | 桌面、Web、CLI 与自动化入口复用同一套定义 | [Host 合同](../../crates/host/a4n-api-contract)、[运行适配](../../crates/host/a4n-runtime)、[业务编排](../../crates/host/a4n-orchestrator)和[Host 装配](../../crates/host/a4n-host-assembly)各有归属；[前端合同检查入口](../../frontend/package.json)维护生成类型与 Web API 一致性。图中箭头表示调用关系，不能当作 Rust 直接依赖图。 |

这些设计共同支持的创作方式是：**保留原文依据，逐步积累资料，围绕当前目标取用，再由作者决定修改与派生作品的更新。** 来源管理、检查点和共享合同是工程机制；运行结果、模型效果和正式发布状态分别核对。

## English

The table maps the README's claims to implementation at the baseline above. Code links identify rules; [validation.json](validation.json) records the narrower set of operations actually exercised in this documentation update.

- **Traceable memory:** key extraction references resolve to chapter source spans. Analysis synchronizes reusable material while preserving author-owned outlines; it does not automatically rewrite the blueprint or part outlines.
- **Task-specific, time-bounded retrieval:** candidates come from the current outline, intent, characters, unresolved threads and scene cues. Generation retrieval is limited to earlier chapters and the matching embedding identity. Context is organized into continuity, summaries and source passages with separate budgets.
- **Reviewable changes:** bounded source reads and content fingerprints support stored patch proposals, visible approval and stale-source checks. The demo verifies the chapter remains unchanged before approval and receives exactly the reviewed edit afterward.
- **Traceable creative branches:** derived projects retain snapshots; chapter and profile differences inform update decisions. Locally edited work has decision guards. A freeform video canvas is a separate workflow, with no implied novel snapshot association.
- **Shared video planning:** frame generation and video clips refer to the same segment plan and execution identity. First-frame chains require the preceding clip's tail frame. These are input and workflow constraints; visual output still requires review.
- **Recoverable work:** completed calls are stored with input fingerprints and reused where the workflow permits. Changed formal sources prevent older generation results from being saved. Shared Host contracts keep entry-point responsibilities explicit.
