# 设计依据 / Design evidence

[中文首页](../../README.md) · [English README](../../README.en.md) · [实际演示与验证](demo.md)

README 的中英文功能图以应用基线 `23ee94632c15b318f8462b23371b51d61c78ae4e` 的现有入口和流程为依据。图表用于说明能力、执行顺序、数据流与作者决策，不代表每个模型都支持所有输入，也不表示所有步骤都自动执行。

公开演示仍对应原应用基线 `e2f3aa0dcdd7a0ab613e986050eae568a9850c56`，其实际验证范围单独记录在 [validation.json](validation.json)；README 的图表更新没有改写该演示记录。正文事实反馈与导演编辑的当前设计分别见[事实反馈](fact-feedback.zh.md)和[导演编辑](director-native-editing.md)。

## 图表与实现入口 / Diagram sources

中英文 README 各有 13 张 Mermaid 图，顺序和节点关系一致；图表直接随文档维护。

| 图表 / Diagram | 实现入口 | 关键边界 |
| --- | --- | --- |
| 功能总览 / Overview | [项目类型](../../crates/platform/a4n-domain/src/project.rs)、[应用入口](../../frontend/src/App.tsx)、[能力包合同](../../crates/host/a4n-api-contract/src/package.rs) | 小说、漫画、章节视频、画布和互动故事有各自工作台；功能包及 Provider 按需配置。 |
| 小说写作 / Novel writing | [灵感](../../crates/host/a4n-orchestrator/src/inspiration)、[蓝图](../../crates/host/a4n-orchestrator/src/blueprint)、[大纲](../../crates/host/a4n-orchestrator/src/outline)、[章节生成](../../crates/host/a4n-orchestrator/src/chapter_generation)、[局部优化](../../crates/host/a4n-orchestrator/src/optimize) | 手工输入和模型生成并存；正文补丁需要作者采纳与版本检查。 |
| 分析与事实反馈 / Analysis and feedback | [章节分析](../../crates/host/a4n-orchestrator/src/chapter_analysis)、[事实反馈](../../crates/host/a4n-orchestrator/src/fact_feedback)、[生成检索](../../crates/host/a4n-orchestrator/src/chapter_generation/rag_context) | 当前事实每章同步，低频归纳按周期；未来剧情调整是建议；检索范围与 Embedding 身份受约束。 |
| 图片与立绘 / Images and portraits | [立绘任务](../../crates/host/a4n-runtime/src/actions/portrait_generation.rs)、[图片与素材](../../frontend/src/api/image.ts)、[立绘接口](../../frontend/src/api/portrait.ts)、[素材依赖图](../../frontend/src/pages/novel-detail/asset-graph/AssetDependencyGraphDialog.tsx) | 生成、上传和使用素材各有入口；素材归属及已保存绑定不会由当前主立绘列表重新推断。 |
| 漫画 / Comics | [漫画编排](../../crates/host/a4n-orchestrator/src/manga)、[阶段与依赖](../../frontend/src/pages/novel-detail/manga/mangaTabUtils.ts)、[参考图投影](../../frontend/src/pages/novel-detail/manga/mangaPagePromptModel.ts) | 分页、单页改编、分镜、参考图和成图有依赖；删除前检查下游影响，系统与用户参考图分别管理。 |
| 章节视频 / Chapter video | [视频编排](../../crates/host/a4n-orchestrator/src/video)、[工作台阶段](../../frontend/src/pages/novel-detail/VideoTab.tsx)、[导演编辑](director-native-editing.md) | 正式导演确认由作者执行；帧图与视频共用片段计划，具体输入取决于工作流和 Provider。 |
| 互动故事 / Interactive stories | [Galgame 编排](../../crates/host/a4n-orchestrator/src/galgame)、[编辑与运行接口](../../frontend/src/api/galgame.ts)、[WebGAL 导出](../../crates/host/a4n-api-contract/src/export.rs) | 分支、台词、演出与素材绑定分别保存；图像生成和音频导入是不同能力。 |
| 自由画布 / Freeform canvas | [画布编排](../../crates/host/a4n-orchestrator/src/canvas)、[节点行为](../../frontend/src/pages/video-studio/canvasNodeKinds.ts) | 图中是可连接的示例；生成节点用于自由视频项目，不替代章节派生视频的工作流。 |
| Agent / Agents | [执行与委派](../../crates/agent/a4n-agent-application/src/agent)、[前端接口](../../frontend/src/api/agent.ts)、[领域控制台](../../frontend/src/components/agent/DomainAgentConsole.tsx) | 领域、资源、权限、模式和审批共同约束执行；未将规划中的 Agent RAG 列为已实现能力。 |
| 来源同步 / Source synchronization | [来源差异与决策](../../crates/platform/a4n-db/src/source_sync)、[同步界面](../../frontend/src/pages/derived/SourceSyncDialog.tsx) | 已编辑的派生内容有决策保护；独立画布不因此拥有小说来源快照。 |
| 协作与交付 / Collaboration and delivery | [成员与邀请合同](../../crates/host/a4n-api-contract/src/collaboration.rs)、[角色](../../crates/platform/a4n-domain/src/collaboration.rs)、[导出格式](../../crates/host/a4n-api-contract/src/export.rs)、[项目包](../../crates/host/a4n-orchestrator/src/project) | 按成员角色访问；阅读格式、可编辑项目包和完整环境备份用途不同，不承诺多人实时光标协同。 |
| 多端架构 / Architecture | [Host 合同](../../crates/host/a4n-api-contract)、[业务编排](../../crates/host/a4n-orchestrator)、[运行适配](../../crates/host/a4n-runtime)、[数据访问](../../crates/platform/a4n-db) | 箭头表示调用或数据流，不是 Rust 直接依赖；模型请求可能使用本地服务或外部 API。 |
| 任务恢复 / Task recovery | [LLM 检查点](../../crates/platform/a4n-db/src/workflow/llm_checkpoint.rs)、[分析恢复](../../crates/host/a4n-orchestrator/src/chapter_analysis/workflow/checkpoint.rs)、[来源指纹](../../crates/host/a4n-orchestrator/src/chapter_generation/service/generation/checkpoint/fingerprint.rs) | 仅支持恢复的流程可复用匹配输入的完成步骤；来源变化不能通过检查点绕过。 |

## 关键机制 / Mechanisms

| 值得关注的设计 | 对创作的意义 | 实现依据与适用范围 |
| --- | --- | --- |
| **关键提取回指原文** | 事实、实体与伏笔有可核对的出处 | [片段引用校验](../../crates/host/a4n-orchestrator/src/chapter_analysis/owner/materialize/values/source_spans.rs)要求引用非空且能在本章原文片段中解析；[实体与事件落地](../../crates/host/a4n-orchestrator/src/chapter_analysis/owner/materialize/pipeline.rs)复用这一规则。引用成立仍需作者判断提取含义。 |
| **分析后继续沉淀资料** | 一次章节分析继续更新后续创作需要的资料 | [分析后的同步流程](../../crates/host/a4n-orchestrator/src/chapter_analysis/application/analysis.rs)处理回顾章纲、分析/伏笔/RAG、物体发现、人物发现、场景和人物资料；[章纲来源规则](../../crates/platform/a4n-db/src/project/outline_records/provenance.rs)保留作者正式章纲。[事实反馈](../../crates/host/a4n-orchestrator/src/fact_feedback)逐章同步事实、按周期归纳世界观等资料，梗概及未来剧情调整保留为作者可采用的建议。 |
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

The diagram index maps all 13 diagrams in each README to the current implementation baseline above. Matching diagrams use the same node relationships in both languages. The public demo remains tied to its original baseline; [validation.json](validation.json) records that demo's narrower scope, not a new full-feature acceptance run.

- **Traceable memory:** key extraction references resolve to chapter source spans. Analysis preserves author-owned outlines. Fact feedback updates current facts per chapter and periodically consolidates knowledge; changes to the premise or future plotting remain proposals for the author.
- **Task-specific, time-bounded retrieval:** candidates come from the current outline, intent, characters, unresolved threads and scene cues. Generation retrieval is limited to earlier chapters and the matching embedding identity. Context is organized into continuity, summaries and source passages with separate budgets.
- **Reviewable changes:** bounded source reads and content fingerprints support stored patch proposals, visible approval and stale-source checks. The demo verifies the chapter remains unchanged before approval and receives exactly the reviewed edit afterward.
- **Traceable creative branches:** derived projects retain snapshots; chapter and profile differences inform update decisions. Locally edited work has decision guards. A freeform video canvas is a separate workflow, with no implied novel snapshot association.
- **Shared video planning:** frame generation and video clips refer to the same segment plan and execution identity. First-frame chains require the preceding clip's tail frame. These are input and workflow constraints; visual output still requires review.
- **Recoverable work:** completed calls are stored with input fingerprints and reused where the workflow permits. Changed formal sources prevent older generation results from being saved. Shared Host contracts keep entry-point responsibilities explicit.
