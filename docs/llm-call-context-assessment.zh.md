# LLM 调用上下文的逐项评估：职责、可用信息与最小充分集合

> 评估日期：2026-10-05。工作树：`work/debug`，HEAD：`53b4520885cc`。以当前磁盘源码及默认 prompt 为依据，与 [LLM 输入/输出字段清单](llm-call-io-fields.zh.md) 配套阅读。本次只分析并新增文档，不修改业务行为。

> 2026-11-07 收尾更新：C15 规范姓名、C17 统计窗口、C25 分镜来源输入已修复并通过定向回归，见[收尾记录](debug-closeout-20261107.zh.md)。其余调用保留原静态评估及证据边界，不将本次修复外推为全部上下文已最小化或真实模型质量已验收。

## 1. 结论与评估口径

**当前上下文设计不能整体认定为“完成各调用功能所需的最小信息集合”。** 一部分调用已经按目标单元、来源片段和前序依赖裁剪；另一部分仍使用整份资料或累计历史。也存在反方向的问题：材料虽然更短，却未必包含完成当前判断所需的原文证据、相关实体状态或相邻边界。

这里的“最小”指**在保持功能正确、来源可追溯和既有输出合同的前提下，候选的最小充分信息集合**，不是字符数最少。源码可以证明某字段被传入、某资料未被传入、某输出依赖哪些输入；不能单凭源码证明删除字段后模型质量不变。本报告没有运行真实模型、读取业务数据库或做消融实验，因此不报告未经测量的 token 节省、成功率或质量提升。

### 1.1 覆盖范围

覆盖字段清单中当前存在的 **49 个生产聊天 LLM 静态调用点**。编号沿用原审查记录，已退役调用不再列出。一个调用点可能按章节、页面、镜头、阶段或 Agent 回合执行多次；恢复命中 checkpoint 时可能没有新请求。共用执行器不重复计数，阶段内部的重要输入差异单独展开。

配置探测及图片/视频生成 provider 不计入该 49 个聊天调用；其模型输入边界在第 12 节补充。运行时重试若没有改变业务输入，不另算新的上下文设计。

### 1.2 如何理解“调用时可提供的所有信息”

逐项的“可用”分三层，均以当前权限、项目关系、来源版本和执行时刻为边界：

- **A：已经持有。** 当前 DTO、已加载记录、已冻结来源、已完成前序阶段、当前历史及工具结果。
- **B：可以读取。** 同一合法作用域内已经存在、可通过现有资料/工具接口取得，但本次请求未加载或未选入的材料。B 是检索候选池，不代表应该全部发送，也不代表当前模型具备读取它的工具。
- **C：此时不可用。** 尚未生成的后续结果、尚未发生的工具结果、未被选择的未来来源版本等。不能把 C 算成当前调用的缺失输入。

为避免每项重复罗列全部数据库字段，下文使用以下资料类别；逐项仍会说明其实际范围、已加载状态和未传入部分。

| 资料类别 | 内容及边界 |
| --- | --- |
| 项目与规划 | 项目题材、描述、写作/媒介目标；蓝图故事核心、世界观、人物与关系；分部大纲、章纲、节拍。规划不等于正文已发生的事实。 |
| 正文与证据 | 当前/既有章节正文、摘要、结构化分析、来源锚点和版本。派生项目优先使用选定原文快照，不默认混入根项目最新正文。 |
| 正式资料 | 角色档案、属性变更、行为、分章状态；物体和场景档案及出场记录；伏笔与事件；有适用范围的已确认正文事实。 |
| 媒介中间产物 | 已采用改编计划、事件/节拍投影、对白、视觉锚点、页面槽位、分镜、视觉计划、拍摄剧本、片段和帧提示词。必须区分前序产物与尚未生成的后序产物。 |
| 素材 | 角色立绘、场景图、物体图、参考图及其文本元数据、绑定关系、已生成媒体。存在图片记录不等于模型看到了图片像素。 |
| 交互与执行状态 | 用户当前指令及更正、历史消息、已选问题、权限、工具目录/结果、Skill 目录、运行状态、摘要、checkpoint、来源 fingerprint。审计/校验元数据不自动属于模型语义上下文。 |

### 1.3 结论标签和证据边界

- **近似合理：** 当前信息范围与任务及输出合同基本相称，未发现明确的业务必需信息缺口；不等于已证明最小。
- **可缩减：** 存在按职责进一步投影、按相关性选择或去重的空间。
- **条件性缺口：** 在条目明确列出的业务情形下，现有输入不足以支撑所要求的判断；不泛化为所有调用都会失败。
- **混合：** 同时有过宽和不足，或者不同阶段/模式结论不同。

每项先写功能与时机，再写 A/B/C、实际输入、候选最小集和证据。**“实际输入”指当前 builder 和默认模板形成的模型可见内容，不把整个业务 DTO、模板变量表或 trace 当成 prompt。** 本报告涉及的不少 manifest 稳定包只有 `full_raw` 单变体并标记 required；这类包不会由预算层自动摘要，required 的最小变体仍放不下时会显式报错。其他调用和 prompt variables 的实际内容仍须按具体 builder 判断。依据：[运行时准备](../crates/host/a4n-runtime/src/dynamic_llm/trace/preparation.rs#L17)、[预算选择](../crates/llm/a4n-llm-context/src/planner/budget/selection.rs#L18)、[消息渲染](../crates/llm/a4n-llm-context/src/manifest/render.rs#L16)。部署时若覆盖默认模板，需要按该模板和最终请求重新评估。

## 2. 小说创作、优化与灵感

### C01 章节正文生成

- **功能/时机/产物：** 在目标章纲及写作参数准备后生成一章正文，供章节保存、后续分析及多媒介改编使用。它需要延续既有事实并落实本章事件，不能只依靠题材和一句任务。
- **可用：** A 包括章纲、用户写作指令、字数要求、蓝图及筛选后的人物/关系/伏笔、主角档案、本次 RAG 结果、前文章节和正式资料投影。B 包括未入选的相关配角状态、物体/场景历史、其他原文证据和规划细节；须按目标章之前的适用状态读取。C 是新章正文及其分析，不能提前提供。
- **实际输入：** `writing` system；`writing_chapter_outline` 内合并章纲、指令、字数、关键参考和 RAG；`writing_context_before` 包含已确认世界事实、角色状态、物体/场景资料、分层摘要、最近 12 章结构化线索、上一章末尾至多 600 字符。默认 system 不会因 variables 中存在章纲而再复制一遍。显式 `context_before` 非空时直接替代自动前情组装。
- **结论：混合。** 已有分层摘要、角色选择和事实优先级，方向合理。前情组装会读取项目内全部角色档案，但输出角色状态行最多 8 个，并按最近状态/档案顺序选择，不按目标章纲人物筛选；物体/场景也是按历史新近程度各最多 8 个，并非由本章实体决定。章纲角色匹配不到时，蓝图人物选择又会退回全部。由此可能带入无关人物并漏掉本章重返的旧场景/旧道具或配角状态。世界观字符截取也不能保证关键限制仍保留。源码能证明选择规则，实际遗漏需结合具体章节验证。
- **候选最小集：** 本章目标/节拍/字数与用户约束；涉及实体在本章之前的最新有效状态及相关世界规则；本章应推进/回收伏笔；压缩的前情因果链与紧邻结尾；只补足上述空缺的 RAG 证据。保留“正文事实优先于未更新规划”。显式前情应被视为调用方承担完整性的输入，而不是默认为自动资料的补充。
- **证据：** [正文准备](../crates/host/a4n-orchestrator/src/chapter_generation/application/generation.rs#L726)、[人物/关系/伏笔选择](../crates/host/a4n-orchestrator/src/chapter_generation/preparation/reference_context/selection.rs#L8)、[前情及显式覆盖](../crates/host/a4n-orchestrator/src/chapter_generation/context_before/resolution.rs#L13)、[项目角色档案全集加载](../crates/platform/a4n-db/src/project/chapter/context/profiles.rs#L192)、[角色状态8行上限及排序选择](../crates/host/a4n-orchestrator/src/chapter_generation/context_before/character_state/query.rs#L6)、[物体选择](../crates/host/a4n-orchestrator/src/chapter_generation/context_before/object_profile.rs#L6)、[场景选择](../crates/host/a4n-orchestrator/src/chapter_generation/context_before/scene_profile.rs#L7)、[写作模板](../prompts/novel/04_writing/writing.md#L5)。

### C02 蓝图优化

- **功能/时机/产物：** 用户要求调整现有蓝图时，依据灵感对话和优化 focus 返回蓝图变更字段，交给后续解析/应用流程；不是继续写正文。
- **可用：** A 有已有蓝图、蓝图人物/关系、灵感历史、focus；准备流程读取规划来源时还能取得正式资料。B 有同项目正文、章纲、正式档案和已确认事实。C 是本次蓝图改动实施后的下游新大纲/正文。
- **实际输入：** 灵感 `messages[{role,content}]` 与 `{blueprint,characters,relationships}` 两个 required/full_raw 包；动态 focus 和变更输出合同。正式正文资料不因此自动进入模型。
- **结论：混合。** 全量灵感和蓝图适合跨模块重设；只改某个角色或语气时可以缩减。若 focus 明确要求“与已写正文/正式角色事实一致”，仅凭灵感和规划无法确认一致性，属于条件性缺口。不能据此要求所有蓝图优化都加载全部正文。
- **候选最小集：** 用户已确认意图、当前 focus、待改字段及其依赖人物/世界规则、必须保留的全局约束；仅在涉及已写剧情时附相关正式事实和来源。历史压缩须保留更正、否决和未决选择，不能只取最后一条 assistant 总结。
- **证据：** [准备与读取范围](../crates/host/a4n-orchestrator/src/blueprint/refine/execution/preparation.rs#L29)、[实际上下文](../crates/prompt/a4n-prompt/src/blueprint_generation/refine/context.rs#L11)、[只返回变更字段及意图优先级](../prompts/novel/02_blueprint/blueprint_optimization.md#L5)。

### C03 局部正文补丁

- **功能/时机/产物：** 选定优化问题/要求后生成局部替换补丁，含用于定位的原文锚点；补丁进入确认流程，不是模型直接改库。
- **可用：** A 有当前完整正文、内容 hash、选中问题和按权限加载的分析上下文；后者可含项目、蓝图、全部分部/章纲、角色、伏笔及前 3/后 2 章摘要。B 是其他合法资料/正文，但不是所有补丁都需要。C 是确认后正文及重新分析结果。
- **实际输入：** 完整正文包、去掉重复正文/导出配置等字段的项目上下文包、选中问题、任务和 `base_content_hash`；system 带权限、能力和相关指令。
- **结论：可缩减，主要应缩项目资料。** 当前合同要求 `old_text` 逐字来自正文且全章唯一，所以整章正文有明确用途，不能直接裁成问题附近几句并声称功能不变。与选中问题无关的全书规划、角色资料则不一定有用。
- **候选最小集：** 改写目标和验收标准、选中问题依据、全文与 hash，以及直接约束该补丁的相关事实。若未来只发片段，必须由确定性代码提供并验证全章定位/唯一性；这属于合同与实现变化，单改 prompt 不足以保证。
- **证据：** [两个上下文包及去重](../crates/host/a4n-orchestrator/src/optimize/patch/request.rs#L15)、[权限内资料加载](../crates/host/a4n-orchestrator/src/optimize/context.rs#L22)、[邻章摘要范围](../crates/platform/a4n-db/src/optimize/context.rs#L261)、[补丁锚点合同](../prompts/novel/05_analysis/content_optimization_patch.md#L5)。

### C04 蓝图分阶段生成

- **功能/时机/产物：** 新蓝图生成顺序为故事核心 → 世界观 → 人物 → 人物关系；阶段 JSON 汇总成蓝图。章纲数组、总章数和分部需求等部分字段由代码补入，不由这四次模型输出共同“猜测”。
- **可用：** A 有标题/题材/章数，条件性存在灵感全文、用户生成指令、conversation_state、progress_summary、next_question、项目投影；第二阶段起有已完成阶段 JSON。B 是同项目其他已有资料，是否适用取决于此次是初创还是对既有项目生成。C 是尚未完成的后续阶段。
- **实际输入：** 每阶段都收到完整 seed；`previous_context` 累计包含全部已完成阶段。动态输入指定当前阶段及输出 schema。
- **结论：可缩减，但须按阶段而非统一截断。** seed 同时包含灵感全文和结构化进度时可能重复；前序结果累计增长。人物和关系确实依赖世界观/故事，不能因为是前序输出就删除。

| 阶段 | 产物及下游用途 | 候选最小依赖 |
| --- | --- | --- |
| story_core | 题材、风格、基调、受众、一句话故事和梗概；约束后续阶段 | 已确认创作意图、题材/篇幅、冲突与结局约束、尚未决的要求。无需尚未生成的人物表。 |
| world | 规则、地点、阵营、历史、文化及伏笔方向 | 故事核心中与世界机制有关的因果/冲突，以及用户明确设定。长篇完整灵感对话可投影。 |
| characters | 人物属性、动机、背景、弧线 | 故事角色职能、相关世界规则/阵营、已确认人物约束；与角色无关的完整地理/历史可筛选。 |
| relationships | 关系、强度和演化 | 确定的人物身份/动机/弧线、全局故事冲突及关系限制；不必默认重复每个人全部外貌。 |

- **证据：** [四阶段累计上下文与系统补值](../crates/host/a4n-orchestrator/src/blueprint/generation/execution.rs#L223)、[seed 与前序包](../crates/prompt/a4n-prompt/src/blueprint_generation/stage_request/context.rs#L11)、[阶段输出字段](llm-call-io-fields.zh.md#28-蓝图生成阶段字段)。

### C05 优化 Agent 决策回合

- **功能/时机/产物：** 章节自动分析或优化对话中的每一步决策；产出结构化动作，读取资料/调用能力/汇总问题，达到可回答或待用户确认的条件后结束。这里生成的是下一步行动，不是默认直接生成整章。
- **可用：** A 有权限内完整 analysis_context、工具目录、base task、已有问题/summary、累计 observations；对话模式还加载既有 turns。B 是工具允许读取/检索的冻结资料，超权限资料不属于可用池。C 是尚未执行工具的结果和未确认补丁的效果。
- **实际输入：** 自动分析首轮只给正文可读取位置、字数和资料目录/数量，**并未预注入整章正文或整份项目资料**。对话模式另投影历史 `{status,user,assistant}`。每步重发 base/rules，并将每个 observation 作为 user 消息完整追加。
- **结论：混合。** 首轮按目录披露、用 path/分页按需读取接近所需最小集；后续完整 observations 和全部对话 turns 累积，已解决的重复查询会持续占窗。不能把首轮未发正文当作缺口，因为存在读取工具。
- **候选最小集：** 当前目标/权限/工具合同、资料目录、有效用户更正、已确认问题、未完成动作/错误、最近必需证据和可重读的资源 ID/hash；已处理的大块结果可归并，但必须保留分页未读范围、失败和依据。当前读工具有 hash 检查、文本/数组分页和 16 KiB 结果上限，这些完整性信号应保留。
- **证据：** [首轮准备](../crates/host/a4n-orchestrator/src/optimize/analysis/request.rs#L39)、[首轮正文读取指引](../crates/host/a4n-orchestrator/src/optimize/messages/analysis_prompt.rs#L5)、[对话历史投影](../crates/host/a4n-orchestrator/src/optimize/messages/chat_prompt.rs#L8)、[逐轮消息累积](../crates/host/a4n-orchestrator/src/optimize/agent_loop/request/envelope.rs#L33)、[读取能力及边界](../crates/host/a4n-orchestrator/src/optimize/runtime_tools/read.rs#L13)。

### C06 章节视觉计划中的镜头段描述

- **功能/时机/产物：** 已有视觉草稿并选中一个镜头段后，生成该段可编辑的变化描述；不是重新规划全章分镜或识别图像内容。
- **可用：** A 有整份当前计划及来源、目标段、关键帧、明确末帧、场景绑定、旧 prompt、用户描述。B 有该来源正文其他部分和相关素材；未输入像素的图片不能当作已观察事实。C 是后续拍摄设计及生成视频。
- **实际输入：** 章节标题/摘要及入场、出场场景投影；本段时长、相对关键帧、场景绑定、旧描述和当前描述。旧描述与用户输入相同时置 null；仅明确指定时才提供目标末帧。
- **结论：近似合理。** 任务只补充本段动作和摄影变化；全章正文、所有场景和后续镜头不属于必需输入。未指定末帧时，不能把下一镜头的画面强行作为本段结尾。
- **候选最小集：** 当前段边界/时长、起点及内部关键帧、必要场景空间资料、用户要求、明确末帧和需保留的旧描述。若用户要求修复仅能从图片看出的细节，需要另行读图结果或像素输入，当前纯文本任务不具备该观察能力。
- **证据：** [目标段投影和去重](../crates/host/a4n-orchestrator/src/chapter_visual_plan/generation.rs#L194)、[只加载入/出场景](../crates/host/a4n-orchestrator/src/chapter_visual_plan/generation.rs#L73)、[镜头边界及图片限制](../prompts/novel/08_video/director_segment_prompt.md#L5)。

### C07 灵感对话

- **功能/时机/产物：** 每次灵感交互时生成回复、下一问题/选项、创作状态和进度摘要；完成态为蓝图生成提供种子信息。
- **可用：** A 有当前用户输入、全部灵感历史、项目投影、可选蓝图和最近章节摘要。B 有更多正文、正式资料和规划，但本调用没有因此获得通用检索工具。C 是用户尚未作出的下一选择及后续蓝图。
- **实际输入：** 项目上下文包、所有 user/assistant 历史及本轮输入。`inspiration_messages_llm_context` 只过滤角色，不压缩 content，也不剥离历史 assistant JSON 中的 conversation_state/summary。
- **结论：可缩减；涉及正文校验时存在条件性缺口。** 保留创作轨迹有价值，但每轮完整重发逐步重复的状态和选项并非唯一实现。仅凭近期摘要不能对久远正文细节作确定性判断，普通构思对话则无需加载全书。
- **候选最小集：** 最新有效创作状态、用户明确约束/否决/更正及依据、未决问题、最近相关往返、当前输入和必要项目边界；回溯旧选择时按需恢复原文。状态与历史矛盾时优先用户更正，不能未经验证直接让旧 assistant 摘要覆盖用户要求。
- **证据：** [请求顺序](../crates/host/a4n-orchestrator/src/inspiration/request.rs#L21)、[项目/蓝图/近期摘要投影](../crates/host/a4n-orchestrator/src/inspiration/context.rs#L6)、[历史 content 原样保留](../crates/inspiration/a4n-inspiration-core/src/lib.rs#L7)。

## 3. 改编、导入与角色资料

### C08 媒体改编计划

- **功能/时机/产物：** 漫画/Galgame 开始改编、缺少已采用计划，或用户要求生成新版计划时，生成 `AdaptationPlanV1`：改编意图、取舍、叙事单元和节拍。后续分页或分支/剧本阶段消费它。
- **可用：** A 有目标媒介、用户指令、派生项目冻结原文及来源映射、可选冻结视频参考、上一版计划。B 有同来源版本的其他章节、正式资料和素材；不能无条件使用根项目最新正文覆盖快照。C 是本次计划尚未生成的页面、分支、剧本行和图片。
- **实际输入：** `{target,instruction,source,chapter:{number,title,content},video_reference,previous_plan}`，正文带短来源标签；dependencies 用于业务依赖管理，不整体发送。
- **结论：近似合理。** 本章计划需要了解整章因果和取舍，完整本章原文比只给摘要更可靠；更新时 previous_plan 的单元/节拍身份有保留用途，不应当作重复历史删除。视频参考如很长，可只投影与改编有关的动作/对白/关系和来源版本。
- **候选最小集：** 当前媒介与改编要求、带来源的本章正文、必须继承的既有单元/节拍及取舍、相关参考的叙事内容。完整摄影参数、执行状态和依赖管理记录不是必需的创作信息。
- **证据：** [调用与保存](../crates/host/a4n-orchestrator/src/media_authoring.rs#L161)、[输入及强制快照来源](../crates/host/a4n-orchestrator/src/media_authoring.rs#L241)、[原文/参考优先级及更新身份](../prompts/novel/06_manga/chapter_media_adaptation_plan.md#L5)。

### C09 Galgame 源同步：目标场景剧本改编

- **功能/时机/产物：** 来源同步对一个场景局部重跑时，先重建该场主线/支线文本行，交给 C10 演出标注；不是整本 Galgame 重新生成。
- **可用：** A 有目标场景的导演投影、所在章完整快照正文、目标来源锚点、场景标题/地点/时段/背景说明及说话人目录。B 有同一派生项目的邻场剧本/边界以及相关正式资料。C 是本次重跑的新演出和视觉规划。
- **实际输入：** 带标记的章节数据、只有当前场景的 adaptation_projection，以及 `branch_design={choices:[]}`。局部重跑 orchestration 明确构造空分支计划，所以该调用没有现存分支选择上下文；后端合同允许保留支线字段，不代表输入中存在支线。旧场景其他字段也不会自动透传进该请求。
- **结论：可缩减，跨场衔接有条件性缺口。** 目标已缩到一场，但正文仍是所在章全文，可收敛为目标场来源及必要前后文。若要求保持跨场人物/舞台状态，当前输入没有显式的邻场演出边界，不能据整章文学叙述保证游戏舞台连续性。
- **候选最小集：** 目标单元/节拍和来源正文窗口、保留的分支/合流约束、必要人物称谓与语气、前后场必须衔接的状态。不能仅靠单元摘要复原应保留的原文台词。
- **证据：** [局部调用](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages/script.rs#L24)、[局部重跑构造空分支方案](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages.rs#L18)、[单场投影但保留章全文](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/preparation.rs#L19)、[剧本输入](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L39)。

### C10 Galgame 源同步：角色演出

- **功能/时机/产物：** C09 后给每一行绑定说话人及完整舞台状态，包括在场人物、进入/退出、位置/朝向、表情/姿态、焦点和玩家认知；输出供视觉规划和游戏持久化使用。
- **可用：** A 有刚生成的剧本及说话人目录，调用链也持有目标场景投影与原文。B 有角色别名/档案、邻场结束舞台状态和已生成立绘。C 是后续视觉资产选择与生成结果。
- **实际输入：** `script_adaptation + speaker_profiles`，这里的 profile 只投影 `{id,name}`；不是完整人物档案，也没有图片附件。局部 WorkflowLlmOnce 路径没有 pipeline 的额外共享 system 投影。
- **结论：近似合理，身份与衔接有条件性缺口。** 固定台词和顺序足以支持场内演出，姓名/ID 足以完成直接同名绑定。台词使用别名或需要沿用跨场在场状态时，姓名目录和本场文本可能不足。无法可靠绑定允许 null，不能把这一合同写成一定能绑定成功。
- **候选最小集：** 本场有序完整剧本、涉及人物的 ID/规范名/别名、必要入场状态、演出枚举和保留字段规则。只有要选现成立绘时才需提供可用表情/姿态素材，当前阶段不必发送所有图像。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages/staging.rs#L24)、[输入 builder](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L68)、[人物目录仅 ID/姓名](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/source.rs#L139)、[逐行完整状态合同](../prompts/novel/09_galgame/character_staging.md#L7)。

### C11 Galgame 源同步：视觉规划

- **功能/时机/产物：** C10 后为目标场景生成背景需求和少量事件 CG 候选行，后续资产流程再渲染图像 prompt；本调用不生成最终图像 prompt 或图像。
- **可用：** A 有目标场景改编投影、完整演出行、原文和源同步输入中的旧场景信息。B 有场景正式档案、已有背景/CG 和同来源版本视觉限制。C 是本次规划对应的新图像。
- **实际输入：** `adaptation_projection + character_staging`；原文全文、既有背景记录和场景档案候选表没有随这两项自动传入。
- **结论：近似合理，视觉事实继承有条件性缺口。** 对“根据当前场景和演出选择背景/高潮行”已具备主要信息。若要求继承既定建筑布局、天气或绑定特定场景档案，而这些未体现在投影/演出中，则不能保证。输出存在可空 `scene_profile_id` 不表示模型得到了合法候选 ID 表。
- **候选最小集：** 目标场景的稳定视觉事实、带行号的剧情/演出重点、合法来源引用；要复用既有场景时补相关档案 ID 和视觉约束。可省略对背景/CG 选择无影响的重复逐行状态，但应保留候选行的完整语义和位置。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages/visual.rs#L25)、[实际输入](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L120)、[背景和 CG 的职责](../prompts/novel/09_galgame/visual_planning.md#L7)。

### C12 导入反推

- **功能/时机/产物：** 对导入内容或已有项目归纳蓝图、章纲或分部大纲，生成候选 JSON，后续验证/应用；一个静态调用对应三种 target。
- **可用：** 手工模式 A 只有明确样本和目标合同；自动模式 A 读取项目/章节/已有大纲，蓝图模式还读取蓝图与正式资料。B 包括已读取但未入选的完整正文、未选章节及完整分析。C 是尚未反推/应用的下游规划。
- **实际输入：** `source_artifact` 来自手工样本或自动投影；有效 JSON 包成 `source_context`，普通文本包成 `source_text`。手工与自动模式不能混称“都看了全书”。

| target | 自动模式实际材料 | 最小性结论及候选集合 |
| --- | --- | --- |
| blueprint | 项目描述、抽样章节摘要、至多 10 章样本（每章正文预览至多 6000 字符）、已有章纲/分部、抽取的人物/伏笔、正式资料、既有蓝图/关系 | **混合。** 来源广有利于归纳全局，但相同事实会出现在抽取、档案、规划多个位置；抽样也可能漏关键反转。候选是覆盖全书关键转折的分层摘要、来源优先级、人物/世界事实及冲突处原文，不能用样本量替代覆盖率。 |
| outline | 标题、全书章节数，以及前 `context_limit` 章的摘要/短预览、少量人物和事件 | **条件性缺口。** 可以归纳已提供章节；`total_chapters` 不是尾部章节内容。缺摘要时正文只取短预览，无法支撑后半章完整章纲。候选是目标每章覆盖全章因果的摘要/节拍；摘要不充分时读取对应全文或分段证据。 |
| part_outline | 前 `context_limit` 条已有章纲，若无章纲则取章节投影；全部已有分部及来源最大章号 | **混合。** 分部划分需要跨章结构，章纲通常足够，不必喂全书原文。若只提供前缀却要求覆盖尾部，仍缺依据；已有分部可只保留相关边界和全局结构。 |

- **候选最小集的共同要求：** 明确 target、实际覆盖章号、来源优先级、已有结构及需保留的身份；输出只对已覆盖来源负责。需要全书结论时先补覆盖，而不是让模型根据总章数补造。当前限制是条目数/抽样/预览规则，不能统称为 token 预算最优选择。
- **证据：** [手工/自动及 provenance](../crates/host/a4n-orchestrator/src/import_reverse/context.rs#L37)、[三目标分流和前缀限制](../crates/host/a4n-orchestrator/src/import_reverse/context/project.rs#L11)、[蓝图投影](../crates/host/a4n-orchestrator/src/import_reverse/context/blueprint.rs#L15)、[10 章抽样](../crates/host/a4n-orchestrator/src/import_reverse/context/blueprint/chapters.rs#L14)、[正文预览](../crates/host/a4n-orchestrator/src/import_reverse/context/chapter_entries/sample.rs#L9)、[章摘要投影](../crates/host/a4n-orchestrator/src/import_reverse/context/chapter_entries/summary.rs#L10)、[实际调用](../crates/host/a4n-orchestrator/src/import_reverse/execution/call.rs#L44)。

### C13 视频提示词优化

- **功能/时机/产物：** 用户对已有视频/帧提示词提出修改时，输出 `visual_prompt/motion_prompt/negative_prompt`，按类型验证后用于更新提示词记录。
- **可用：** A 有原提示词记录、类型/模式/时长/画幅、用户优化指令及输出语言。B 有该来源的帧/片段计划、参考图与绑定、拍摄剧本、邻片段和已生成媒体；这些在当前优化路径中未整体加载给模型。C 是按新提示词重生成的媒体。
- **实际输入：** 原记录的标题、摘要、三类 prompt、prompt_type/workflow_mode/shot_number/duration/aspect_ratio；动态仅当前 instruction，system 规定语言和输出合同。不含原记录 metadata 全量、参考图像素或邻片段。
- **结论：近似合理，依赖外部素材的改写有条件性缺口。** 纯措辞、语言、描述强化不需整章。要求“与参考图一致”“衔接上一个镜头”时，只有现有 prompt 不能观测真实参考图/邻片段。
- **候选最小集：** 原 prompt、改写目标和不可变媒介约束；按指令补被点名的帧/实体/邻段状态或读图结果。镜头编号等可留作定位，不能替代上述语义材料。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/video/prompts/optimization.rs#L380)、[当前提示词投影](../crates/host/a4n-orchestrator/src/video/prompts/optimization.rs#L452)。

### C14 场景正文生成

- **功能/时机/产物：** 在某章内生成或重写指定 scene 的正文，归一化后保存/返回，供章节场景编辑使用。
- **可用：** A 有章节标题/摘要、可选完整章正文/章纲、全部既有 scene（包括目标旧内容）、scene_number 与用户 prompt。B 有正式角色/场景资料和邻章事实。C 是新场景正文及其后续分析。
- **实际输入：** chapter 基础信息加 `available_context`：章全文、章纲、排除目标后的所有 scene 全文、单列目标旧 scene；动态是目标编号与指令。
- **结论：混合。** 章全文与逐场内容在已合成章节时可能语义重复，不应无差别双发。另一方面，这里没有自动继承 C01 的正文事实/实体状态选择；当章内上下文不足而任务要求遵守跨章设定时会缺资料。
- **候选最小集：** 当前场景目标/章纲片段、旧场景（重写时）、前后场边界和相关实体状态、必要本章全局走向。整章与逐场文本应按权威版本投影；若两者版本不同，必须保留来源区别，不能机械当重复删除。
- **证据：** [实际上下文与目标排除](../crates/host/a4n-orchestrator/src/chapter_scene/context.rs#L48)、[执行与输出](../crates/host/a4n-orchestrator/src/chapter_scene/execution.rs#L265)。

### C15 角色冲突检查

- **功能/时机/产物：** 用户指定角色及可选章节区间，比较档案、分章状态、属性事件和章节摘要，返回带 expected/actual 来源 ID 的矛盾项及严重度。
- **可用：** A 有目标 profile（含规范名）、范围内 ChapterRecord、状态时间线、属性事件；因此正文在准备阶段已可取得。B 有范围外的前置状态及相关证据，但不得将范围外变化误当成范围内当时事实。C 是尚未执行的修复/重新同步结果。
- **实际输入：** `{character_name,chapter_scope,character_profile:{source_id,base_attributes,current_state},chapter_context,character_state_timeline,attribute_events}`。章节只取摘要/关键事件/metadata，没有摘要的章被跳过；属性事件包含 evidence。总内容超过 24000 字符显式拒绝。
- **结论：主体身份缺口已修复，证据范围仍有边界。** 规范姓名现在进入稳定上下文和请求指纹，回归检查实际发送的模型消息，旧请求不能仅凭相同属性复用指纹。本功能仍只能审查摘要/状态已表达的冲突，不能据空结果断言正文无矛盾；当前最新 profile 与历史区间相比仍应注意时间语义，避免把正常成长当冲突。
- **候选最小集：** 明确角色规范名/别名；检查范围和区间起点有效状态；范围内该角色相关的状态变更及带时间、source_id 的原文证据；只补必要章节背景。源码已经持有身份，不需要增加新检索就能提供。正文相关片段应覆盖待比对事实，不能把整章无关人物摘要一并当作目标角色行为。
- **证据：** [已加载 profile 与实际 role_attr](../crates/host/a4n-orchestrator/src/character/conflict/execution.rs)、[规范姓名进入稳定上下文](../crates/host/a4n-orchestrator/src/character/conflict/request.rs)、[实际模型消息及指纹回归](../crates/host/a4n-orchestrator/src/character/conflict/tests.rs)、[摘要选择/跳过及来源粒度](../crates/host/a4n-orchestrator/src/character/conflict/context_rules.rs)。

### C16 角色档案同步五阶段

- **功能/时机/产物：** 对指定角色的连续章节范围同步档案，依次生成事件、属性变化、分章状态、关键行为和删除候选；后续分类/复审和确定性投影负责归并。模型不直接给最终属性快照或执行删除。
- **可用：** A 有目标角色、范围内完整正文及锚点、范围起点 baseline、已完成阶段结果。重跑旧范围时会读取起点之前状态，不简单采用最新快照。B 有更早的证据、其他人物资料；仅在消歧/关系判断需要时相关。C 是后续阶段结果及尚未完成的分类/复审。
- **实际输入：** 五阶段都重发完整带锚点章节和 baseline；前序依赖已裁剪：事件无前序；属性/行为只接事件；分章状态/删除候选接事件和属性变化。
- **结论：可缩减，已有正确的阶段依赖边界。** 完整原文对首轮事件发现有覆盖价值；后续所有阶段继续收到同一整段原文和全部 baseline，并非各自最小依赖。不能改成只发事件摘要而失去原文校验，尤其是推断属性与删除提议。

| 阶段 | 产物用途 | 候选最小集 |
| --- | --- | --- |
| events | 后续同步的事实事件/行为线索 | 角色身份、范围内可完整发现该角色活动的正文、锚点；baseline 只需身份/已有关键事实。先按姓名硬切正文会漏代词、别名和场外影响。 |
| attribute_changes | 显性/隐性/社会属性 add/modify | 事件及对应原文、涉及属性 baseline、assertion 和类别规则；无关属性/无关正文可去除。 |
| chapter_states | 每个选定章节结束状态 | 基线状态、逐章事件、属性变更、必要结束状态证据和完整章号清单；不能漏无变化章节。 |
| behaviors | 为隐性分类提供关键行为及原文锚点 | 已发现事件、行为所在原文/必要前后因果、角色身份；不需要整个社会属性集合。 |
| deletion_candidates | 对失效属性提出有证据的删除候选 | 现有相关属性、明确否定/失效证据、事件和本次变化；未提及不等于失效。 |

- **证据：** [范围和基线准备](../crates/host/a4n-orchestrator/src/character/profile_sync/context.rs#L146)、[基线回溯](../crates/host/a4n-orchestrator/src/character/profile_sync/context.rs#L206)、[共享原文包](../crates/host/a4n-orchestrator/src/character/profile_sync/stages/request.rs#L48)、[按阶段挑前序](../crates/host/a4n-orchestrator/src/character/profile_sync/stages/request.rs#L152)、[实际调用](../crates/host/a4n-orchestrator/src/character/profile_sync/stages/execution.rs#L260)。

### C17 隐性属性复审

- **功能/时机/产物：** 用户手动复审某属性，或同步复审中某属性的不符合行为达到阈值时，判断 keep/modify/delete，返回 reasoning、新值和支持行为 ID；后续流程处理建议及证据。
- **可用：** A 有目标属性当前值、已分类行为记录、窗口/阈值、统计和受 evidence_limit 限制的证据列表。B 有列表外已读取行为和原文更大范围。C 是复审后的新状态与未来行为。
- **实际输入：** 目标角色/属性、窗口参数、统计、选中 evidence 的描述/原文/标签/分类；没有全角色全部属性，也没有整个章节，范围基本聚焦。
- **结论：统计窗口边界已修复，证据选择仍需与统计范围区分。** 共享 helper 仅在进入窗口之外的新章节时停止，窗口 2 的 `[4,4,3,3]` 四条相关行为现在都会参与统计；窗口最后一章的非法分类也不能被提前退出隐藏。手工与同步复审共用同一修复，证据数量上限不截断统计。evidence 仍只取顺序前若干条，不能要求模型为未见行为给出细节依据。
- **候选最小集：** 目标属性及适用时间、明确覆盖范围的正确统计、足以支持或反驳变化的代表性正反行为原文和 ID；提供证据抽样/覆盖说明。对矛盾、反讽、被迫行为只在需要时补局部因果，不能仅凭分类计数断言人格改变。
- **证据：** [自动阈值筛选](../crates/host/a4n-orchestrator/src/character/implicit_review/execution.rs#L157)、[手动复审也调用同一统计 helper](../crates/host/a4n-orchestrator/src/character/implicit_review/execution.rs#L244)、[统计窗口与 evidence 截取](../crates/host/a4n-orchestrator/src/character/implicit_review/stats.rs#L48)、[实际字段及只能引用已见证据](../crates/host/a4n-orchestrator/src/character/implicit_review/request.rs#L105)、[LLM 入口](../crates/host/a4n-orchestrator/src/character/implicit_review/execution.rs#L340)。

### C18 行为隐性属性分类

- **功能/时机/产物：** 同步出关键行为后，逐行为判断与当时隐性属性 conform/non-conform，并提出新属性候选，供统计与 C17 复审使用。没有现有隐性属性时此路径跳过模型分类。
- **可用：** A 有行为原文/描述/类型/标签、本次属性变化及按章推进的 current implicit attributes。B 有同章其他行为和相关上下文；C 是后续行为及复审结果，不能拿未来角色状态倒灌本次分类。
- **实际输入：** 当前隐性属性对象、目标角色名、一个行为的章号/描述/标签/原文。执行前先应用章号不大于该行为章号的属性变化。
- **结论：近似合理，因果语境有条件性缺口。** 逐行为、只发隐性属性符合该功能；全角色外貌、世界设定通常无关。若证据是一句表面行为而真实动机在相邻段落，可能误判。当前按章节应用变化，也不表达同章内变化前后顺序，不能把它描述为精确到行为时刻的状态。
- **候选最小集：** 行为及能解释其动机/语境的最短原文窗口、当前有效隐性属性、身份和分类规则。并非每个属性都必须有结论；无适用属性应返回空 classifications，避免为补齐标签而引入无关信息。
- **证据：** [逐行为状态推进和空属性跳过](../crates/host/a4n-orchestrator/src/character/implicit_classification/execution.rs#L49)、[输入与空分类合同](../crates/host/a4n-orchestrator/src/character/implicit_classification/request.rs#L102)、[LLM 入口](../crates/host/a4n-orchestrator/src/character/implicit_classification/execution.rs#L142)。

## 4. Galgame 完整流水线

这四处与 C09–C11 的单场源同步分别计数。A 的公共材料是整个选定快照中的章节、已采用改编计划、来源锚点和人物姓名/ID 目录；各阶段额外拥有已完成的前序结果。场景投影由代码从改编计划生成，**不是额外一次 LLM 调用**。依据：[加载范围](../crates/host/a4n-orchestrator/src/galgame/read_models/pipeline.rs#L53)、[改编计划准备](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/preparation.rs#L47)、[确定性场景/节拍投影](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/scene.rs#L37)。

### C19 分支设计

- **功能/时机/产物：** 改编场景就绪后生成玩家选择、选项行动/反应和合法合流点，保存为分支计划，供 C20 改编支线。匹配依赖的已保存计划可直接复用，此时不调用模型。
- **可用：** A 为公共材料及既有可编辑分支计划；B 为同快照全文细节、角色动机资料和已有游戏演出；C 是新主线/支线台词及演出。
- **实际输入：** 全部 adaptation_projection 放在共享 system 数据中，user 通过 `$ref` 引用，另有分支规则和合同；不再独立复制同一投影，也不传全文或旧分支计划。
- **结论：近似合理。** 设计分支需要跨场顺序和后续合流位置，不能只给当前节拍而删除所有后续单元。已采用计划包含戏剧目标/阻力和节拍，用其设计新分支通常足够；若动机/世界限制没有投影，才需补相关资料。
- **候选最小集：** 可分支节拍的目标/冲突/状态、相关人物动机、合法后续合流节点及其前置条件、玩家认知限制。远处场景可只保留合流所需的结构摘要；不能省去节拍身份和序列关系。
- **证据：** [复用条件与模型入口](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/branch.rs#L58)、[分支输入](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L158)、[共享 system 组装](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/llm/stage/preparation.rs#L63)。

### C20 剧本改编

- **功能/时机/产物：** C19 后将全部选定场景与分支改编成主线/支线剧本行，保留 beat_ref、来源和顺序，交给 C21。
- **可用：** A 为公共材料及已确定 branch_design；B 为角色别名、语气等正式资料和同版本补充事实；C 为演出状态、背景/CG 选择及实际素材。
- **实际输入：** 全部选定章节的带标记全文、分支设计，以及共享 system 中的完整 adaptation_projection；来源记录由短标签投影，不另重复发送全部 source_spans 记录。
- **结论：可缩减。** 原文对台词和叙述改编有必要，但一次把所有章节和所有场景交给一个请求，范围随作品长度增长。可以按单元/场景生成，同时保留全局分支结构与相邻衔接；这需要保持跨场合同，不能简单截断全文。
- **候选最小集：** 目标场/分支的节拍和原文窗口、人物称谓/必要语气、入口/出口状态、选择与合流合同、必要全局剧情摘要。是否必须保留整章而非窗口，取决于指代和跨场因果覆盖。
- **证据：** [业务调用](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/script.rs#L49)、[全文与分支输入](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L39)、[快照正文标记](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/source.rs#L27)、[source_spans 不重复发送](../crates/host/a4n-orchestrator/src/source_markers.rs#L20)。

### C21 角色演出

- **功能/时机/产物：** C20 后对全部主线/支线逐行输出完整舞台状态，并绑定人物 ID；为游戏行记录和 C22 提供演出数据。
- **可用：** A 为完整剧本、人物目录、场景投影及公共材料；B 为别名、相关人物资料/已有素材；C 为本次新背景/CG。
- **实际输入：** user 中是完整 script_adaptation 和 `{id,name}` 人物目录；pipeline 还在 system 中发送全部 adaptation_projection。与 C10 的区别是多了这份共享场景数据，且处理全部场景。
- **结论：可缩减。** 剧本行必须可访问以保留台词、判断逐行变化；已写入剧本的节拍和来源无需再用全部导演细节重复解释。共享投影有助于起始人数/场景目标，但可以只保留剧本未表达的舞台约束。别名歧义仍需相关别名映射，不能只增大人物目录。
- **候选最小集：** 目标场有序剧本、涉及人物 ID/规范名/别名、进入场景的完整舞台状态、分支入口及合流状态、枚举/字段保留合同；可按场处理，但不能丢分支在场状态。
- **证据：** [user 与额外 shared_adaptation_projection 同时提供](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/staging.rs#L33)、[共享数据实际成为 system 消息](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/llm/stage/preparation.rs#L63)、[演出保留合同](../prompts/novel/09_galgame/character_staging.md#L87)。

### C22 视觉规划

- **功能/时机/产物：** C21 后为各场背景和重要剧本行生成 CG 需求；后续持久化与资产流程消费，最终图片 prompt 由专用模板生成。
- **可用：** A 为全部演出行、场景投影及公共材料；B 为场景档案和已有背景/CG 资产及其适用版本；C 为将要生成的新媒体。
- **实际输入：** 共享完整 adaptation_projection，user 引用它并附完整 character_staging；没有场景档案候选表或图片附件。
- **结论：混合。** 全部逐行完整 presentation 对选择少量 CG 可能过宽；既有场景视觉事实若未写进投影/演出又可能缺失。场景 ID 可空，不能把模型输出该字段理解为已完成可靠的既有场景匹配。
- **候选最小集：** 各场稳定地点/时间/光线/重要道具与来源；可成为 CG 的剧情行及周边情绪/动作、角色位置和行号；需继承时附相关正式视觉约束/合法 ID。完整逐行演出可以作为可检索材料，而非每次全量输入。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/late_stages/visual.rs#L42)、[实际输入](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L120)、[背景/CG 输出职责](../prompts/novel/09_galgame/visual_planning.md#L7)。

## 5. 漫画

本节保留调用清单的稳定编号。角色、对白、视觉锚点、物品和章节综合五个旧调用已合并为 C28 漫画预处理；当前执行顺序为：C08 改编计划 → C28 预处理 → C26 分页 → 每页 C24 改编/C25 分镜/C29 参考图发现/C27 绘图提示词。C23 是独立的提示词优化。

流水线 A 的公共材料为统一来源解析后的本章正文/锚点、已采用漫画改编计划、角色主立绘引用文本、已完成阶段结果。B 是同来源版本的其他正式资料和素材；后续页面/图片尚未生成时属于 C。依据：[来源、上下文及执行顺序](../crates/host/a4n-orchestrator/src/manga/pipeline/run/pipeline/execution.rs#L44)、[从改编计划投影事件](../crates/host/a4n-orchestrator/src/manga/adaptation_projection.rs#L29)。

参考立绘实际为 `{profile_id,portrait_id,character_name,image_path,prompt,portrait_type,expression,pose,is_primary}` 文本 JSON，**不是像素附件**。只有路径不能证明模型看到了图。依据：[素材投影](../crates/manga/a4n-manga-core/src/lib.rs#L11)。各阶段 input 字段包均 required/full_raw，不能假定预算不足时自动变成摘要。

### C23 漫画提示词优化

- **功能/时机/产物：** 用户修改已生成的某页绘图 prompt 时，输出新的 `{image_prompt}`，供该页后续重绘使用。
- **可用：** A 有提示词记录、dialogue_text、页码和用户要求。B 有本页分镜、页面改编、参考立绘、原文和已有图片。C 是此次重绘的新图。
- **实际输入：** 原 image_prompt/dialogue_text、prompt_id/page_number、instruction 和输出合同；没有额外项目包或图片附件。
- **结论：近似合理，图像诊断有条件性缺口。** 对措辞、密度、风格修改已聚焦；保留 dialogue_text 有防止文字丢失的用途。若用户要求修复“图片里画错了什么”，只有旧 prompt 不能观察画面。
- **候选最小集：** 旧 prompt、必须保留的台词、修改要求；涉及布局时补本页分镜，涉及视觉错误时补实际图片/读图证据。内部 prompt_id 主要用于定位，不是生成新文本的语义依据。
- **证据：** [实际请求](../crates/host/a4n-orchestrator/src/manga/prompt_optimization/request.rs#L17)、[调用](../crates/host/a4n-orchestrator/src/manga/prompt_optimization/execution.rs#L160)。

### C24 单页改编计划

- **功能/时机/产物：** 分页后，为一个页面确定视觉意图、连续性要求、保留/省略细节及 beat_refs，交给 C25；不决定页数或画格。
- **可用：** A 有公共材料、已完成抽取、所有 page slots 和当前页/邻页内容。B 有相同来源的补充细节；C 为当前页分镜和生成图片。相邻页内容已存在，但相邻页实际分镜不保证已经生成。
- **实际输入：** 当前页 content、仅本页 beat_refs 对应的单元/节拍、页码/总页数、前后页 content；不含整章原文和完整抽取集合。
- **结论：近似合理。** 本调用解释已确定的剧情合同，按页和节拍裁剪是正确边界。无需让它再次从全文决定取舍；若上游未保留关键视觉事实，本调用不能凭空恢复，应补对应证据而非整包扩张。
- **候选最小集：** 当前页目标/动作/反应/出入状态、相关节拍的必要性与意图、相邻页边界。全局规划记录 ID 可留在 trace；剧情身份 beat_refs 必须保留。
- **证据：** [单页入口](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page.rs#L602)、[当前页/邻页输入](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/page_pipeline.rs#L53)、[仅选本页节拍](../crates/host/a4n-orchestrator/src/manga/adaptation_projection.rs#L69)、[页面内容结构](../crates/platform/a4n-domain/src/manga/contracts/pipeline.rs#L410)、[职责模板](../prompts/novel/06_manga/manga_page_adaptation_plan.md#L7)。

### C25 单页分镜

- **功能/时机/产物：** C24 后生成当前页 `MangaPageV4`，含阅读顺序、画格、构图/动作、文字层及其位置；C27 将这些转成绘图 prompt。
- **可用：** A 有页内容、页改编、节拍、邻页状态、本章 characters 和按参与者筛选的立绘文本。B 是相关正式资料/已有图；C 是该页图像，不能据它规划本次分镜。
- **实际输入：** 页内容、页改编、相关节拍、邻页内容、characters 和立绘文本，以及 `source_context` 中按本页来源 ID 选取的原文、事件、对白与旁白；不整包传 visual_anchors 或 items。
- **结论：原句输入缺口已修复。** 分镜执行、预览及阶段指纹复用同一本页来源投影，未知来源明确报错；模型可依据原文与 speech 选择文字层，不能用 `dialogue_intent` 自造台词。候选文字不要求全部上屏。定向回归证明原句到达模型且其他页内容不混入，不等于真实模型的文字还原质量已经验收。
- **候选最小集：** 当前页剧情/视觉合同、相邻页出入状态、本页相关人物/素材；**当页面需要原句时，按 beat_ref 选入对应对白/旁白和必要来源原文**，以及当前画格要表现的场景/道具事实。来源标记和已选对白不能仅留在前序 trace 中。若改成允许模型新写台词，应明确改变业务合同，不能再按“只引用原文”评估。
- **证据：** [本页来源投影](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/inputs.rs)、[统一模型请求输入](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/page_pipeline.rs)、[原句与页范围回归](../crates/host/a4n-orchestrator/src/manga/tests/messages.rs)、[文字层来源规则](../prompts/novel/06_manga/manga_page_storyboard.md)。

### C26 页面槽位/分页

- **功能/时机/产物：** 抽取和综合完成后决定页数、每页剧情 content 及来源/节拍覆盖，产出 slots，再逐页进入 C24；不生成画格。
- **可用：** A 有本章正文、改编计划及全部角色/对白/视觉/物品/综合输出；B 是相关正式资料与既有页面历史；C 是本次单页改编、分镜和图片。
- **实际输入：** 五类 extraction_summary、adaptation_plan 和带标记全文。`manga_page_slots_source` 内同时含原始 units 和从同一 units 派生的 events，二者叙事信息重叠。
- **结论：可缩减。** 确定跨页压力、台词密度、required beat 覆盖确实需要全章改编结构，但未必需要每个角色全部外貌/性格、物品中英文完整描述；units/events 可保留一个规范表示。原文是复核取舍的证据，不应在未验证摘要覆盖时一概删去。
- **候选最小集：** 全章有序节拍、必要/高潮标记、出入状态、场景转换、对白内容或容量、阅读/页数约束、相关来源；仅在选择/切分有歧义时加原文窗口。应确保分到页的具体对白能继续传给 C25，不能只把对白信息压成意图后丢失原句。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page_slots.rs#L32)、[全文和抽取输入](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/page_pipeline.rs#L18)、[units 与 events 同时投影](../crates/host/a4n-orchestrator/src/manga/adaptation_projection.rs#L59)。

### C27 单页绘图提示词

- **功能/时机/产物：** 当前页分镜完成后生成 `{schema_version,page_number,image_prompt}`，后续图像 provider 消费 prompt 和系统绑定的参考图。
- **可用：** A 有当前完整分镜、页面内容/改编、全部抽取资料和立绘文本；B 有已生成图片像素、相关场景/物品素材与邻页视觉结果（存在时）；C 是本次新图像。
**实际输入：** 当前页分镜、页面内容和改编要求；按参与者筛选的 characters/立绘；系统发现的参考图、用户补充参考图及可用图片描述。图片路径和描述是文本资料，不等同于 vision_images 附件。
- **结论：近似合理。** 页面分镜和两组参考图都是绘图提示词的直接输入。参考发现输出与用户绑定分别保留，避免把用户补充误当成系统选择。不能据文本资料断言模型看到了参考图像素或实际成图。
- **候选最小集：** 当前页分镜和文字层、必要页面连续性、相关角色视觉规范、系统/用户参考图的用途说明、风格和画幅。仅在要求判断像素级一致或修复具体图像错误时补实际图像附件。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/manga/pipeline/run/drawing_prompt/page.rs#L46)、[页级 payload](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/page_pipeline.rs#L143)、[人物与素材筛选](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/inputs.rs#L105)、[参与者来源](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/page_context.rs#L9)。

### C28 漫画改编前预处理

- **功能/时机/产物：** 在改编计划之前一次性整理原文事件、人物、对白/旁白、视觉锚点、物品和章节情绪/篇幅估计；不生成改编节拍或分页。
- **实际输入：** 带来源标记的整章正文和角色参考资料文本；输出中的来源 span 与事件引用由合同校验。
- **结论：合并合理。** 五类旧抽取/综合调用有共同的全文事实来源，合并后少了重复上下文和中间请求。职责仍由单一预处理产物统一提供给改编计划与分页。
- **证据：** [调用](../crates/host/a4n-orchestrator/src/manga/pipeline/run/preprocessing.rs#L38)、[上下文](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/preprocessing.rs#L9)、[合同](../crates/platform/a4n-domain/src/manga/contracts/pipeline.rs#L49)、[提示词](../prompts/novel/06_manga/manga_preprocessing.md#L1)。

### C29 单页参考图发现

- **功能/时机/产物：** 分镜完成后独立识别本页要维持的视觉参考要求，再从可用图片描述中可选匹配系统参考图；无图片候选时仍输出需求，只有素材匹配列表为空。
- **实际输入：** 当前页内容、改编、分镜、相关人物和候选图片描述；用户补充的图片从匹配候选中排除，文字需求与实际系统/用户图片随后分别传入绘图提示词。
- **结论：** 语义需求识别与素材匹配分开。资产类型和 ID 必须来自候选，服务端会校验并补回权威路径/描述。图片元数据是文本，不能声称模型查看了图片像素。
- **证据：** [调用和候选集](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page.rs#L800)、[候选校验](../crates/host/a4n-orchestrator/src/manga/reference_assets/discovery.rs#L45)、[用户输入](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/page_pipeline.rs#L165)、[提示词](../prompts/novel/06_manga/manga_page_reference_discovery.md#L1)。

## 6. 拍摄剧本

执行顺序是：已确认导演视觉计划 → 代码生成全章骨架 → C35 单元布局 → C34 表演事实 → C33 逐镜头设计 → 组装拍摄剧本。骨架生成本身没有 LLM。

三个调用 A 都持有统一来源版本、本章全文/锚点、已确认视觉计划及投影、绑定场景档案和可能存在的旧拍摄剧本；B 可含同作用域相关角色/物体资料和素材；具体后序设计属于 C。每次模型只收到目标单元原文摘录、章节标题/摘要/导演意图、入/出场景资料及该阶段附加包，不会因为准备 fingerprint 包含全章而收到全章。依据：[准备范围](../crates/host/a4n-orchestrator/src/shooting_script/preparation.rs#L18)、[实际来源/场景筛选](../crates/host/a4n-orchestrator/src/shooting_script/context.rs#L46)。

### C33 拍摄设计镜头

- **功能/时机/产物：** 单元布局和表演事实完成后，每次为一个已锁定镜头生成景别/机位/运镜、调度、表演、声音和首尾状态。虽然返回 `ShootingSceneV2`，`shots` 只能有目标镜头，随后由代码合并为场景。
- **可用：** A 另有整个单元计划、全部锁定镜头、本镜头相关表演事实；顺序循环中已完成的镜头设计也已经存在，但 request builder 不接收它们。B 是已有拍摄脚本中需维持的实际设计状态；C 是未设计镜头和实际视频。
- **实际输入：** 单元原文/场景公共包；去掉重复 performance_facts 的 screenplay_plan_unit、完整 locked_visual_unit、target_shot、仅本镜头 required_performance_facts。
- **结论：混合。** 已按镜头限定任务和表演事实，但 locked_visual_unit 包含其他镜头，target_shot 也在其中出现；原文仍以单元而非目标镜头取范围，可继续筛选。若前一镜头刚确定的相机方向/轴线并未写在锁定计划中，本次看不到该新决定，只能独立推断连续性。
- **候选最小集：** 本镜头锁定时间/场景/关键帧及用户导演描述、相关表演事实与原文、单元戏剧/转场约束、相邻镜头必须继承的相机/人物/道具/声音边界。用明确边界替代整段已完成设计；不能删掉来源或导演新增事实身份。
- **证据：** [逐镜头调用与结果合并](../crates/host/a4n-orchestrator/src/shooting_script/generation/design.rs#L57)、[请求只取计划而不取已完成 shots](../crates/host/a4n-orchestrator/src/shooting_script/generation/design.rs#L141)、[locked unit 只移除一个重复来源字段](../crates/host/a4n-orchestrator/src/shooting_script/context.rs#L144)、[连续性要求](../prompts/novel/05_analysis/chapter_shooting_design_unit.md#L125)。

### C34 表演事实改编

- **功能/时机/产物：** C35 后为本单元提取可观察/可表演的事实，逐条绑定已确认 shot_id、正文来源或 `director:<segment_id>` 新增来源，供 C33 覆盖。
- **可用：** A 另有已细化的目标 layout unit、锁定镜头及其导演描述、允许来源集合。B 是同来源版本相关人物动机/道具状态；C 是详细摄影设计及视频。
- **实际输入：** 本单元原文、场景资料、`unit + locked_visual_unit + allowed_source_span_ids + allowed_user_addition_ids`。不会把前序所有单元结果一起发来。
- **结论：近似合理。** 输出必须把事实归属到具体镜头，镜头序列、导演描述和合法来源 ID 都有直接用途。这里保留正文与导演描述不是无意义重复：两类事实来源不同，不能相互冒充。
- **候选最小集：** 目标单元的戏剧/人物/出入状态、该单元锁定镜头及相关原文、两类来源白名单。无需全部项目规划或未来镜头成片；若关键事实在上游来源切分之外，应补对应证据，不能凭摘要新增情节。
- **证据：** [单元调用](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/adaptation.rs#L38)、[请求与双来源合同](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/adaptation.rs#L82)、[职责模板](../prompts/novel/05_analysis/chapter_screenplay_unit_adaptation.md#L7)。

### C35 剧本单元布局

- **功能/时机/产物：** 代码锁定全章骨架后，逐单元细化标题/场景行、人物、戏剧问题/目标/阻力、出入状态和转场合同；交给 C34。局部重生成满足条件时可复用已有布局，不产生模型请求。
- **可用：** A 有骨架、当前视觉单元、相邻视觉单元、旧拍摄剧本及公共资料。B 为来源范围外相关连续性事实；C 为本次表演事实/详细镜头。
- **实际输入：** 去掉各 unit 重复 source_span_ids 的全章骨架、当前 locked_visual_unit、前一单元末镜头和后一单元首镜头，另有目标单元公共原文/场景包。
- **结论：近似合理，少量可去重。** 全章骨架主要是身份/顺序/时间/剪辑类型，不能把它误评为全章剧本全文。邻单元只取边界镜头，有明确转场用途。directing_statement 同时出现在公共章节包与骨架内，可保留一处。
- **候选最小集：** 当前单元身份与不可变编排、全章相关顺序/交叉剪辑关系、单元原文和空间事实、相邻入/出镜头、导演意图。相邻完整剧情和全部正式资料不是默认必需。
- **证据：** [确定性骨架及复用条件](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/layout.rs#L19)、[模型入口](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/layout.rs#L121)、[骨架/相邻首尾投影](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/layout.rs#L169)、[公共导演意图](../crates/host/a4n-orchestrator/src/shooting_script/context.rs#L81)。

## 7. 正文分析与正式资料

### C36 角色发现

- **功能/时机/产物：** 对指定章或所选已有章节识别真实角色、别名、重要度、三类属性和证据锚点；后端完成身份去重并返回新建/已存在角色，供后续同步和媒介复用。
- **可用：** A 有有效章节正文、当前锚点、项目全部已有角色记录；B 有既有分析、关系/世界资料等，但本任务不能把规划中的人物当成已在正文出场。C 是本次新建档案的后续同步状态。
- **实际输入：** 所选章节带锚点全文、已有角色身份投影。投影包括姓名/别名、性别/年龄、外貌/背景/性格、base/current explicit attributes、last_synced_chapter 和 attribute_counts；不是所有角色记录原样序列化。
- **结论：可缩减。** 原文对发现与消歧必需，已有身份表能避免重复。所有角色详细性格/背景、属性数量和多份重复外貌不一定服务于“是否同一人”；可以先给身份目录，只有候选冲突时增加特征。按姓名预筛正文会漏未识别别名，不能当作安全默认。
- **候选最小集：** 完整覆盖本次发现范围的正文/锚点、规范名/别名与足以区别同名人的身份特征、输出范围规则。可按章增量发现并复用统一身份目录；跨章合并必须由既有去重规则承担，不能让分批产生重复角色。
- **证据：** [加载与选择](../crates/host/a4n-orchestrator/src/character/discovery/execution.rs#L119)、[身份投影](../crates/host/a4n-orchestrator/src/character/discovery/identity.rs#L11)、[实际输入/后端去重职责](../crates/host/a4n-orchestrator/src/character/discovery/request.rs#L83)、[模型入口](../crates/host/a4n-orchestrator/src/character/discovery/execution.rs#L264)。

### C37 场景档案候选/同步

- **功能/时机/产物：** 逐章识别本章实际进入/停留/发生行动的可复用地点，输出 scenes 及已有场景/父场景绑定、空间描述和证据，进入正式场景资料流程。当前 LLM 阶段只产 scenes，不产 occurrences/transitions/entity_states。
- **可用：** A 有本章正文/锚点、全部既有场景、角色资料和物体资料；B 有更早出场记录/状态来源及相关素材；C 是本次候选落库后的新记录和媒体。
- **实际输入：** existing_scene_profiles（含描述、布局、氛围、current_state 和出现章范围）、available_characters（ID/姓名/外貌）、available_object_profiles（ID/名称/类型/视觉/材质状态/位置等）、当前章；previous_stages 当前为空。
- **结论：可缩减。** 既有场景 ID/别名/层级线索对复用和 parent 绑定必要；全项目人物外貌、物体详细状态并非识别每个地点都需要。旧章节重新分析时，最新 current_state 还应与该章原文事实区分，不能直接当作当时布局。
- **候选最小集：** 本章空间行动原文及锚点、全部或高召回的场景身份目录、相关候选场景的稳定空间特征/适用版本、能消歧地点的少数人物/物体关系。场景层级和来源不能为了少 token 而删掉。
- **证据：** [公共资料加载](../crates/host/a4n-orchestrator/src/scene_profile/application/context.rs#L15)、[投影字段](../crates/host/a4n-orchestrator/src/scene_profile/application/context.rs#L167)、[当前只有 scenes 阶段](../crates/host/a4n-orchestrator/src/scene_profile/application/stages/request.rs#L25)、[模型入口](../crates/host/a4n-orchestrator/src/scene_profile/application/stages/execution/call.rs#L37)、[实际出场规则](../prompts/novel/05_analysis/scene_profile_candidates.md#L41)。

### C38 章节分析四阶段

- **功能/时机/产物：** 章节有正文后提取事实、生成摘要、整理角色结束状态、确认伏笔/关键事件；汇总为 analysis_data，服务写作前情、RAG、资料同步及事实回写。
- **可用：** A 始终有当前完整章及来源锚点；facts 完成后其 JSON 可供后三阶段；伏笔阶段另读取项目伏笔资料。B 有前章状态、正式档案/蓝图和历史正文，但它们不是“从本章提取事实”一律必需的输入。C 是未完成阶段、未来章和本次分析之后的正式资料变更。
- **实际输入：** 四阶段共用带短标签本章全文包，user 的重复 content 被删除。summary 收全部 facts；character_state 只收 characters/event_candidates；foreshadowing 只收 event/foreshadowing 候选和已筛选伏笔。

| 阶段 | 产物及作用 | 结论与候选最小集 |
| --- | --- | --- |
| facts | 实体、事件/伏笔候选、world_facts 及 assertion/来源 | **近似合理。** 全文是发现新事实的主要依据；不能先只选已知事件窗口，否则遗漏新事件。需要标记来源及区分 observed/inferred/reported 等，不必注入整份蓝图诱导“按规划补事实”。 |
| summary | compressed/one_line/keywords，以及标签/基调/时间标记 | **可缩减。** 原文可帮助保留情绪/叙述基调，facts 可提供重点；完整实体来源数组等未必有用。候选是关键事件/实体、时间与情绪证据、覆盖叙事主线的原文或经过验证的等价材料。不能因已有 facts 就假定全文完全冗余。 |
| character_state | 本章出场角色的结束位置/状况/变化/情绪 | **近似合理。** 角色候选、相关事件和正文足以提取本章明确状态；模板允许未知为 null、无变化为 []，因此未传上一章 baseline 不自动构成缺陷。若要输出“完整继承后的跨章状态”，才应另补前章状态，这超出当前提取合同。 |
| foreshadowing/events | 新埋/推进/回收伏笔及最终关键事件 | **可缩减。** 需要本章证据和既有伏笔身份，不能只给新候选。候选池按 setup/payoff 章号过滤，但名字虽叫 pending_foreshadowing，实际可含 resolved/abandoned 等合法状态；不要误写成仅未解决伏笔。可保留相关完整项加其余简短身份目录，需验证远期回收召回率。 |

- **总体结论：混合，以保持证据完整性为前提。** 后三阶段仍完整重发本章是明确的成本来源，但可否改成片段取决于对应任务；事实发现、摘要、状态提取不可使用同一个粗暴截断器。
- **证据：** [共享来源包](../crates/host/a4n-orchestrator/src/chapter_analysis/owner/requests/context.rs#L22)、[summary 输入](../crates/host/a4n-orchestrator/src/chapter_analysis/owner/requests/stages/summary.rs#L14)、[状态输入](../crates/host/a4n-orchestrator/src/chapter_analysis/owner/requests/stages/character_state.rs#L14)、[状态未知允许为空](../prompts/novel/05_analysis/chapter_character_state.md#L39)、[伏笔来源和筛选规则](../crates/host/a4n-orchestrator/src/chapter_analysis/owner/requests/stages/foreshadowing.rs#L48)、[实际结构化调用](../crates/host/a4n-orchestrator/src/chapter_analysis/stages/runner/call.rs#L36)。

### C39 正式物体发现

- **功能/时机/产物：** 从指定章/所选有效正文发现可复用物体，生成视觉、材质状态、故事/视觉/生成用途及证据；后端 upsert 正式档案和证据。它与漫画预处理中的章节物品摘要不同，是正式资料入口。
- **可用：** A 有正文/当前锚点、项目全部既有物体档案；B 有相关场景/角色资料、历史物体证据和参考图；C 是本次归并后的档案及新素材。
- **实际输入：** 带锚点章节，以及已有物体名称/别名/类型/重要度、外观/材质状态、三种用途、位置及首次/末次出现章号。没有把实际图片发给模型。
- **结论：可缩减。** 完整正文对新道具发现和状态证据必要；全量既有档案有防重复价值，但每个物体完整生成用途、视觉用途未必需要。相关实体既有状态不能无条件裁掉，否则可能把同一物体改名另建或混淆新旧状态。
- **候选最小集：** 本次正文/锚点、物体身份目录、匹配候选的有效外观/材质/位置和版本；按需补同名物体区别。可分章运行，但必须保留后端统一 upsert 和证据校验，不由模型自己“记住全库”。
- **证据：** [加载与选择](../crates/host/a4n-orchestrator/src/object_discovery/execution.rs#L113)、[实际投影](../crates/host/a4n-orchestrator/src/object_discovery/context.rs#L8)、[模型入口](../crates/host/a4n-orchestrator/src/object_discovery/execution.rs#L243)、[下游 upsert](../crates/host/a4n-orchestrator/src/object_discovery/persistence.rs#L16)。

### C40 正文事实归纳/反馈

- **功能/时机/产物：** 达到归纳间隔、分部终点、出现新约束、来源移除或用户强制时，把当前有效正文事实归纳成资料 changes。后端校验来源和 before 值后应用允许的事实变更；未来规划/梗概调整按合同形成建议，随后同步索引。
- **可用：** A 有所有当前有效反馈来源及 revision、已归纳范围、配置、蓝图/角色/分部等规划来源；本次有 pending 时选择 pending 章节，没有 pending 时选择全部当前来源。B 有未选当前来源的正文证据及更完整正式资料；C 是本次应用后的新版本和索引。
- **实际输入：** 当前蓝图 id/version/梗概/世界观；全部角色 id/name/current_state/voice_fingerprint/character_arc；全部分部摘要/焦点/事件及 planning_rule；所选章节带标记全文和 world_facts。
- **结论：可缩减，但不能只留 world_facts。** 全部角色/分部可能超出本批事实相关范围；可以按来源实体和变化候选选择。原文用于分辨传闻/假设、核实说话风格与角色成长：仅保留世界事实列表会使 character_voice 等输出失去依据。`before` 必须对应实际输入，因此目标记录当前值不可省略。
- **候选最小集：** 当前来源/有效时间、待反馈事实及其原文（含归纳 voice 所需真实对白）、受影响目标的 ID/版本/现值、必须保留的规划边界及变更合同。对未涉及角色只保留可检索目录；不能自动把未来终点当作已发生状态回写。
- **证据：** [触发、pending 选择与实际 payload](../crates/host/a4n-orchestrator/src/fact_feedback/request.rs#L28)、[模型调用及下游校验应用](../crates/host/a4n-orchestrator/src/fact_feedback/run.rs#L253)、[变更类型/真实对白/before 及规划限制](../prompts/novel/05_analysis/project_fact_consolidation.md#L7)。

## 8. 视频提示词

C41/C42 是两处静态渲染/恢复入口，按运行时 prompt family 展开为分段计划、3 种帧规划、4 种视频规划和单独插入片段。阶段根据源 shot、segment/frame、之前已完成 shot、参考素材和工作流模式装配请求。字段清单 §2.10 列出各 payload 和输出合同；本节评估各材料是否匹配任务。

### C41 新请求渲染

- **功能/时机/产物：** 帧/视频计划的阶段请求首次执行时构造实际 system/user 和 manifest、调用模型、解析校验 JSON；产物是分片/镜头计划或生成媒体用文字 prompt。来源不含图片像素，除非另有真正多模态附件，但这些 prompt renderer 不设置 `vision_images`。
- **可用：** A 有选定原章节快照正文/摘要/分析、拍摄剧本投影、已完成前序片段/帧/shot、素材元数据、风格/语言/时长/画幅和章节分析抽取角色；B 可有相关正式档案/更多资产与画面；C 是当前待生成的下游图像/视频。project_context 只在有项目蓝图/人物或角色档案时出现，人物名可从章节分析筛选；若无法抽出名字则纳入全部已知人物。
- **实际输入及最小性：**

| family | 当前输入 | 结论与候选最小集 |
| --- | --- | --- |
| segment_plan | 当前 source shot 及 action_beats、当前 shot 来源正文摘录、压缩后的当前拍摄剧本场景/相邻首尾状态、摘要、style/duration/aspect | **近似合理。** 一次规划一个 shot 的一个 segment，时间/节拍/ID 映射应保留；当前 shot 正文与摘要可有重复，但来源摘录为核对节拍提供依据。可只保留摘要中与本 shot 有关内容。 |
| frame_prompts | 聚焦片段计划、当前 frame、可用素材/参考槽位、必要 source excerpt、压缩章节摘要/分析 | **近似合理。** 单帧任务需要当前段、关键帧和邻帧边界；无关章节人物/远处素材可裁。引用素材的说明只是文字元数据时，不能声称有视觉检查。 |
| first_frame_prompts | 首个 segment、可用素材、章节投影、源摘录 | **近似合理。** 仅首段起点和项目视觉规范必要，不需后段全部时间线；源摘录可限定该段。 |
| storyboard_prompts | 当前 story board frame、聚焦计划、可用素材/槽位、来源摘录及摘要/分析 | **近似合理。** 目标是本视频单元的一张多格故事板；只需当前帧和相邻连续性，不应向该请求输入全部镜头计划。 |
| chapter_video_prompt / first_frame_video_prompt / chat_prompt | 当前 segment、聚焦计划、上一完成 shot、参考素材摘要、输入帧、style/语言/时长/画幅 | **混合。** 这些信息服务跨 shot 衔接；项目 context 中全蓝图 world_setting 可能较大，而视频 shot 常只需要当前角色/场景规范。按当前段角色、场景筛选世界/人物约束，保留 previous shot 边界和真实输入帧约束。 |
| storyboard_video_prompt | 当前 segment、聚焦计划、上一 shot、分镜板、素材摘要和约束 | **近似合理。** 分镜板必须在当前 prompt 明确作为生成依据；无需重复普通首尾帧列表。仅传分镜图片引用/说明而未传像素时，需保证素材管线确实将图像送入 provider，不能从文本 ID 推断模型看见图。 |
| inserted_segment | 章节摘要/正文 excerpt/hash、时间线 revision/gap、前后 item、用户指令、风格语言、时长选项和参考 image ID | **混合。** 时间线邻项是插入连续性的必需信息，revision/hash 用于正确性；但 payload 含项目/章/timeline/item/reference IDs 等业务键，其中多数可由服务端追踪而非生成语义输入。图像 ID 同样不提供像素，若要求视觉延续则应有绑定参考描述/图像。 |

- **公共结论：** 正常流程已有较好的单 shot/单帧/单 segment 聚焦；`video_project_context` 对当前角色有名可按名过滤，否则全人物目录，缺失角色名时容易过宽。source 包对常规路径只含所选来源片段，不重复摘要和分析；此前输出与项目上下文包标记 required/full_raw，不能把 helper 名 `optional` 理解成预算会自动淘汰。应优先审查筛选正确性和 JSON 重复字段，而不是泛化要求删掉图片/视频的上下文。
- **证据：** [分片规划循环/shot scope](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/staged_segment_plan/shots.rs#L112)、[shot 的动态输入](../crates/host/a4n-orchestrator/src/video/prompts/context/builders/planning.rs#L10)、[相邻边界和来源摘录](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/frame_chain/boundary_frames/request.rs#L17)、[项目人物筛选](../crates/host/a4n-orchestrator/src/video/prompts/context/project_context.rs#L13)、[包加入与 required 行为](../crates/host/a4n-orchestrator/src/video/prompts/context/ledger/manifest.rs#L12)、[成片 prompt 输入](../crates/host/a4n-orchestrator/src/video/prompts/context/builders/boundary_video.rs#L15)、[内部 ID 投影](../crates/host/a4n-orchestrator/src/video/prompts/context/public_context.rs#L6)、[入口及 checkpoint](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/render_restore.rs#L64)。

### C42 checkpoint 恢复或再次渲染

- **功能/时机/产物：** 与 C41 共用入口；先按 workflow/run/call key 和输入 fingerprint 查完成 checkpoint。命中时重新按当前请求解析/校验已保存原文，不发生新模型生成；未命中时调用 C41 同一 prompt family。恢复后保留原响应和 usage。
- **可用：** A 同 C41，再加 checkpoint 的原请求/响应/解析产物、既有页/shot 阶段和当前来源版本。当前代码会重建请求用于 trace，并核对 fingerprint/解析后的 checkpoint data。B 同 C41；C 仍是后续生成物。
- **实际输入与结论：近似合理。** “恢复路径”本身没有独立的最小上下文，复用时输入是历史模型响应而非把新内容再次发给模型。若 fingerprint 漏掉改变语义的上下文，旧结果可能错误复用；当前报告未逐项审计每个家族的全部 fingerprint 字段，因此不能宣称 checkpoint 一定覆盖所有语义输入。
- **候选最小集：** checkpoint identity/fingerprint 应包含模型可见内容、prompt/template 版本、来源快照、输出合同和所有会改变解析/业务含义的变量；实际模型输入与 C41 完全相同。checkpoint 数据本身是产物恢复依据，不应再次包装成普通语义 prompt。
- **证据：** [恢复时重建请求、校验并直接返回](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/render_restore.rs#L183)、[无 checkpoint 时才发请求](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/render_restore.rs#L270)、[插入片段独立 payload](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/inserted_segment.rs#L191)。

## 9. Agent 与 Runtime

### C43 Agent 主回合

- **功能/时机/产物：** Agent 每个决策回合选择回答、继续思考或调用工具；模型输出经 `native_tool_calling` 或 `json_compat` 解码后，交给控制面执行。项目资料主要由工具按需读取。
- **可用：** A 有用户/系统消息、当前 Run/Turn/Profile/Scope/Goal、历史、摘要、委派验收条件、在途执行、工具结果、Skill 搜索目录和运行时配置。B 是工具权限范围内能读取的资源、尚未加载的 Skill 正文。C 是新工具执行结果和后续用户回复。
- **实际输入：** system 角色/能力域/范围/协议规则/工具 schema/Skill metadata/权限，以及现存摘要、未覆盖边界后的持久化消息和当前阶段的持久化控制面事实；预算拟合保留 system/当前任务/完整工具调用批次。原生工具定义在 `tools[]`；兼容协议把工具说明放 system。Skill 目录只有 metadata，正文由 load tool 按需取。
- **结论：混合。** 按范围和工具渐进披露比预载所有项目资料更合适，用户/工具原始消息和权限需保留。随着多轮增长，未压缩历史可能重复；摘要必须保留最新更正、失败、资源版本、pending 和未验收 delegate。模型 context fit 会报告被省略历史，主流程随后负责摘要，不能默默删除。
- **候选最小集：** 角色/协议、权限/Scope、当前目标和验收条件、有效摘要、当前用户请求、未完成执行/委派状态、完整最近工具批次及相关历史；其他内容按需读。工具 schema 是调用正确性所需，但不必把超出 profile/scope 的工具授予模型。
- **证据：** [系统消息与摘要/任务状态注入](../crates/agent/a4n-agent-runtime/src/engine/prompt.rs#L3)、[历史选择及工具批次拟合](../crates/agent/a4n-agent-runtime/src/engine/history.rs#L40)、[省略历史时触发压缩](../crates/agent/a4n-agent-runtime/src/engine/loop.rs#L576)、[Agent 主调用](../crates/agent/a4n-agent-runtime/src/engine/loop.rs#L757)、[协议与输入输出字段](llm-call-io-fields.zh.md#L1)。

### C44 Agent 历史压缩

- **功能/时机/产物：** 主回合拟合后有历史被省略，或输入占可用容量达到 80% 时，尝试把一个连续历史前缀压缩为持久摘要；后续主回合复用摘要并保留最新批次原文。若完整工具批次无法装入摘要请求，系统显式报错。
- **可用：** A 有上一份摘要、当前 turn 用户请求、尚未覆盖的历史及工具 observations/执行 ID；预算只允许装入完整工具批次的连续前缀，并保留最新完整批次给主回合。C 是尚未发生的工作。
- **实际输入：** 一条 JSON user：`previous_summary,current_request,messages,covered_through_sequence`；专用 system 规定保留约束/纠正/未完事项/资源与工具结果，失败未知不得写成完成，且校验覆盖边界。
- **结论：近似合理，摘要压缩仍需可检索。** 对已覆盖连续前缀做一次聚合避免逐轮摘要日志；重要原始事实可从历史分页核实。由于压缩器有输入预算，它没有读取的尾部消息不属于本次缺失信息；必须精确记真实 watermark，不能越过未压缩批次。当前 turn 的 user 消息会单列在 `current_request`；若该消息也位于未覆盖的 `messages` 前缀中，则可能重复进入摘要请求，可按 message ID 去重并保留正确覆盖边界。
- **候选最小集：** 上一摘要、当前请求、未覆盖完整消息批次、准确覆盖边界；摘要字段保留有效指令、确认决策、证据/资源版本、已完成、pending 和 tool state。不要把所有细枝末节留在 prompt，但须使可追溯原文仍有稳定 ID。
- **证据：** [摘要输入选择和 watermark](../crates/agent/a4n-agent-runtime/src/engine/history.rs#L117)、[摘要 system/schema/调用](../crates/agent/a4n-agent-runtime/src/engine/context.rs#L28)。

### C45 Agent 图片视觉评估

- **功能/时机/产物：** Agent 明确要求检查图片/视频帧时，按当前用户目标对实际图片像素做一次只读分析，返回视觉缺陷和是否建议重新生成。
- **可用：** A 有目标图片 ID、所属项目/可选章节、当前 instruction、实际解码图像。B 有该资产来源 prompt、相邻帧、剧情/拍摄计划和页面上下文；本调用不自动读取这些文字材料。C 是分析后可能产生的新图像。
- **实际输入：** 系统安全规则、用户 instruction/默认视觉质量要求及 `vision_images` 的图片像素。ID 只用于授权/查找/trace，不作为提示词文本。
- **结论：条件性缺口。** 对孤立图像质量审查，已实际传图且任务清楚，输入足够。若要评估“是否符合对应分镜/原计划/前后镜头连续性”，只给 ID 和画面不够，因为任务没有计划、资产 prompt 或邻帧；需要相关文本/参照图或先明确为孤立图片质检。
- **候选最小集：** 用户检查目标、待看图片像素；若判断遵循剧情/分镜/连续性，再加对应规范和最邻近边界图，不加载整项目资料。多图要说明比较顺序/关系，避免孤立逐图判断。
- **证据：** [图片读取/项目章节校验及多图上限](../crates/host/a4n-runtime/src/actions/image_records/reading.rs#L16)、[实际模型消息和图片附件](../crates/host/a4n-runtime/src/actions/image_records/reading.rs#L83)。

### C46 Canvas 文本素材草稿

- **功能/时机/产物：** 用户在画布请求新建文本素材草稿时，生成可编辑正文；有参考资产时固定引用一个明确版本。
- **可用：** A 有用户指令和可选 `(source_asset_id,source_version)` 精确版本正文。B 有项目蓝图、连接节点和其他资料，但当前 action 没有加载。C 是后续画布节点和用户编辑。
- **实际输入：** 固定文本创作 system、instruction、可选 source content，并要求只返回正文。
- **结论：近似合理，跨项目事实约束是条件性缺口。** 无来源的自由创作应只由指令决定；若任务要求遵循项目世界观/指定人物，当前上下文不足，因该 action 不接收那些资料。不能因它们在 DB 存在就当作模型可见。
- **候选最小集：** 任务/文体/长度等用户约束、被引用的确切版本、明确要求遵守的项目或节点相关事实。版本 ID 用于确定内容；正文而非 ID 提供生成语义。
- **证据：** [来源版本读取、组 prompt 和 LLM 调用](../crates/host/a4n-runtime/src/actions/canvas_workflow/text_asset_draft.rs#L76)。

### C47 Canvas LLM 文本节点

- **功能/时机/产物：** 工作流到达 LlmTextNode 时，按配置转换上游文本，输出交给下游节点；不是自由读取任意项目资料。
- **可用：** A 是有向连接输入文本、节点 instruction/output_kind/温度/token 限制；B 是图中没有连接的节点/项目资料，节点合同禁止访问。C 是下游节点执行结果。
- **实际输入：** 上游文本包、节点配置包、纯文本输出格式及当前节点职责规则；graph/node/run/config identity 在 source metadata/trace 中。
- **结论：近似合理。** 两个 required 包就是当前节点合同所需内容，没发现合理理由增加未连接图节点。上游文本可能很长；应由工作流设计拆分/裁剪，但本调用不能自行猜测哪些段落可删除。
- **候选最小集：** 完整必要上游内容、节点指令、输出类型和下游可用性合同、数值生成约束。可从 trace 剥离内部定位 ID，不应从模型输入删掉实现确定变换所需的文本。
- **证据：** [请求包与职责边界](../crates/host/a4n-orchestrator/src/canvas/text_generation/request.rs#L35)、[Canvas 执行入口](../crates/host/a4n-orchestrator/src/canvas/text_generation/execution.rs#L477)。

## 10. 大纲与立绘

### C48 分部大纲

- **功能/时机/产物：** 按项目生成一个目标分部的大纲，串行承接已完成的前序分部；模型产出标题/摘要/主题/关键事件/人物焦点，代码补分部号和章范围。
- **可用：** A 有项目蓝图或可用正式资料、所有已有 chapter outlines/parts、分部计划和原始 source_snapshot。B 有章节正文和已完成章节分析，当前请求没有直接传整章。C 是之后会生成的分部和新正文。
- **实际输入：** `outline_blueprint_context` 包含蓝图/项目、蓝图人物与关系、全部 formal_materials（人物/物体/场景/未回收伏笔）和全部已有章纲；另有当前生成路径提供的 `previous_parts`、当前分部号/起止章/每部章数/请求总部数。`previous_parts` 随路径变化：续生成/追加传已有或本批已生成部分，预览重生成可置空。prompt 明确正式资料与章纲约束优先。
- **结论：可缩减。** 为设计某一部而带入所有正式物体、场景和伏笔、当前范围外的章纲偏宽；追加路径中的较早分部也可压缩为主线/结尾边界。当前范围已有章纲必须归纳，前部结尾和跨部主线必须保留。原始 source_snapshot 参与 fingerprint/写入时版本锁定，不等于模型可见正文。
- **候选最小集：** 全书主线/世界规则压缩投影、目标分部范围内已有章纲、前一部及相关主线节点、当前活跃人物/伏笔、明确用户优化方向和本分部字段合同。其他部只保留递进/悬念边界；不相关场景/道具目录可筛除。
- **证据：** [加载范围和正式资料](../crates/host/a4n-orchestrator/src/outline/context/loading.rs#L17)、[章纲实际并入 blueprint payload](../crates/host/a4n-orchestrator/src/outline/context/loading.rs#L134)、[manifest 前序 parts 包](../crates/host/a4n-orchestrator/src/outline/parts/requests/manifest.rs#L13)、[请求构造](../crates/host/a4n-orchestrator/src/outline/parts/requests/request.rs#L13)、[连贯性合同](../prompts/novel/03_outline/part_outline_single.md#L55)。

### C49 节拍映射生成一批章节大纲

- **功能/时机/产物：** 用户选择一个已确认分部和 beat_sheet 后，将给定节拍映射到该分部各章，产物为一批章纲；不能自行增删/改写节拍身份。
- **可用：** A 有 beat_sheet、目标分部和完整大纲 context（项目/蓝图、正式档案、全部已有章纲/分部）。B 有相关正文和事实可读，但默认并未作为正文传入。C 是章节正文、事实分析及其他分部后续生成结果。
- **实际输入：** 与 C50 共用 `outline_blueprint_context` + 所有 parts；当前 user 额外带目标分部/范围、整份非空 beat_sheet 和目标章数。温度较低，但仍是全局完整 context。
- **结论：混合。** beat_sheet 和 active part + 目标范围必需；整本已有章纲、所有正式材料可能远超本分部映射所需。另一方面，若跨部伏笔或人物约束会影响目标节拍，不能只发 beat_sheet 而丢掉必要全局约束。
- **候选最小集：** 原样保留的目标节拍、目标分部合同、同范围已有章纲/相邻边界、相关角色/伏笔/规则的摘要及输出章数。全局主线用短摘要保留；没有被节拍/范围引用的全书章纲与无关档案可不传。
- **证据：** [准备完整 context 和 target part](../crates/host/a4n-orchestrator/src/outline/chapters/beat_mapping/preparation.rs#L22)、[beat_sheet 输入及共享请求](../crates/host/a4n-orchestrator/src/outline/chapters/beat_mapping/request.rs#L14)、[全量 context loader](../crates/host/a4n-orchestrator/src/outline/context/loading.rs#L68)、[节拍映射规则](../prompts/novel/03_outline/outline.md#L30)。

### C50 单章大纲

- **功能/时机/产物：** 为指定单章生成一条大纲（或按某入口请求多章共用该 builder）；本调用生成章纲字段，不写正文。
- **可用：** A 有请求中的章号/范围/focus_part、项目与蓝图、全部正式资料、所有分部和所有既有 chapter outlines。B 有相关正文可读取但未在这个调用预取；C 是本章正文与分析。
- **实际输入：** blueprint_context 包已含全部已有 chapter_outline；独立 `outline_part_outlines` 又提供全部分部。动态 user 指定 start/count/end 和可选当前 focus part。不能把“缺前后章纲”列成已发现问题：它们确实在 blueprint payload 里。
- **结论：可缩减。** 当前同一章纲数据同时承担当前范围约束与全局 continuity，而完整正式角色/物体/场景资料、所有不相关分部和章纲过宽。输出每章只要求标题/摘要，其他细节可选，未必需要全部实体 detail。若请求是未来章且要求遵守当前事实，人物/伏笔相关正式资料仍需保留。
- **候选最小集：** 单章目标/范围、目标分部概要、相邻章纲、相关主线摘要、涉及角色当前事实/弧线、相关待回收伏笔和用户 focus；从完整已有章纲中只保留邻章和节拍引用的远期边界。保留用户允许的 beat/key_scene 输出精度，不能为了压缩退化成只有一句剧情摘要。
- **证据：** [已有章纲装入蓝图 payload](../crates/host/a4n-orchestrator/src/outline/context/loading.rs#L159)、[全部 part 包](../crates/host/a4n-orchestrator/src/outline/chapters/requests/context.rs#L13)、[实际 request](../crates/host/a4n-orchestrator/src/outline/chapters/requests/request.rs#L13)、[输出合同和章节范围规则](../prompts/novel/03_outline/outline.md#L11)。

### C51 人物初始三视图提示词

- **功能/时机/产物：** 按角色档案和生成立绘参数，生成可直接交给图像模型的中文三视图/拆解设定图 prompt。使用 FullBody 类型；图片由后续独立媒体 provider 生成。
- **可用：** A 有目标 profile 的姓名/性别/年龄/外貌/性格/背景、portrait_profile_context 派生的 base/current attributes、character_arc、voice_fingerprint（存在时）、项目视觉上下文、用户 style/pose/expression 和已有立绘数量。B 有角色相关章节原文、参考图片像素、完整蓝图细节；当前请求未读/未传。C 是图片结果。
- **实际输入：** 稳定 profile/project 包 + prompt payload 中三视图、拆解区、必需视角/布局、detail focus/policy、负向要求；有 art_style/表情/已有数量。实际 named path 将用户 style/pose 合成为 style_with_pose，portrait_type 固定 FullBody。
- **结论：混合。** 三视图和拆解图合同较详细，能约束图像结构；`background`/`personality` 对造型可能有帮助。档案 context 还会带入 `character_arc` 和 `voice_fingerprint`，它们对静态造型的直接用途较弱。project_context 是蓝图的精简投影，但其中 `world_setting` 可保留较完整的 JSON，可能含与本角色造型无关的规则。旧图片仅以数量进入请求，模型没有旧图像素，因而无法据此检查既有造型一致性。
- **候选最小集：** 规范姓名、外貌/年龄/性别中已确认字段、当前衣着/配饰/持有物、所需视角和拆解组件、风格/构图/画幅限制；仅保留能影响造型的档案条目，除非用户明确要求角色弧线/语气视觉化。把项目上下文筛为时代/世界视觉规则；若用户要求对齐某张已有立绘，需向具有视觉能力的模型提供该图像或经确认的视觉规格。
- **证据：** [named path 当前实际设置](../crates/host/a4n-orchestrator/src/portrait/application/request.rs#L34)、[稳定包和 dynamic payload](../crates/host/a4n-orchestrator/src/portrait/prompt_generation/request.rs#L50)、[输出任务和“不编造”约束](../prompts/novel/07_protagonist/character_portrait_generation.md#L24)。

### C52 单次立绘提示词

- **功能/时机/产物：** 对指定角色 ID 与 PortraitType 生成一次绘图 prompt，输出纯文本；不是图片生成调用。
- **可用：** A 有该 profile、角色上下文和可选项目上下文，以及 portrait_type。B 有相关原文、同一角色的其他立绘像素和用户补充描述，但通用 `PortraitPromptSpec` 的 art_style/expression 字段为 `None`；另外传给模板的 variables 仍可能含其他指令。C 是实际图片及生成后的质量评估。
- **实际输入：** 角色姓名/档案外观/性格/背景及 profile_context、project_context，portrait_type 对应的 FullBody/HalfBody/Face/Chibi 类型文字、通用图像结构/字段约束以及调用方传给模板的 variables。温度 0.8，纯文本输出。
- **结论：混合。** 已把输出范围和角色资料提供给模型；档案 context 包含 base/current attributes、character_arc 和 voice_fingerprint，后两类并非每次都与静态造型相关。project_context 是从蓝图选择 title、genre、style、tone、target_audience、one_sentence_summary 和 world_setting 的投影，其中 world_setting 仍可能较大。没有旧参考图，模型不能视觉核对多张立绘间的造型一致；模板要求缺失事实不要编造，是合理边界。
- **候选最小集：** 本次类型/视角与用途、已确认角色外观及当前版本、相关视觉世界规则、调用方实际传入的模板变量和负向限制。需要保持跨图一致时，应提供主视觉参考图或稳定造型摘要；没有新增要求时无需带完整剧情梗概或所有角色关系。
- **证据：** [单次输入准备](../crates/host/a4n-orchestrator/src/portrait/application/request.rs#L77)、[通用请求包](../crates/host/a4n-orchestrator/src/portrait/prompt_generation/request.rs#L20)、[portrait_type 输出用途](../crates/host/a4n-orchestrator/src/portrait/prompt_generation/request.rs#L112)、[提示词要求](../prompts/novel/07_protagonist/character_portrait_generation.md#L20)。

## 11. 结论与优先级

静态源码审查确认 49 个调用点中同时存在范围适当、可缩减和条件性缺口；“当前最小”没有一个适用于全部调用的答案。事实发现、原文改写、角色身份和来源引用需要足够证据，不能仅因 prompt 很长就删除。页面/镜头/工具阶段已有按单位和权限投影的，应以合同所需依赖为边界继续缩减。

**本轮已核销的明确源码问题（2026-11-07）：**

1. C15 规范姓名进入模型稳定上下文及指纹，已有实际模型消息与身份变化的回归。
2. C25 按本页来源提供对白、旁白与原文；预览和执行共用构建，已有页范围及未知来源回归。
3. C17 完整统计窗口最后一章的相关行为；手工与自动复审共享修复，已有单章、多章和非法分类回归。

**可由 builder 直接证明的重复/过宽候选：**

- C28 将五个旧漫画抽取/综合调用合并为一次预处理调用，减少了重复上下文和中间请求；需持续关注合并后一次响应的 token 规模。
- C04 每个蓝图阶段重发完整 seed 和累计全部已完成阶段输出；按阶段依赖压缩，而不是全局统一截断。
- C16 五阶段持续携带同一整段章正文和 baseline；按事件/属性/状态/删除合同分阶段投影，证据需可回链。
- C20–C22、C48–C50 会在局部目标里附较多全局场景、人物、章纲或分部资料；可按当前目标、相邻边界和相关实体筛选，同时保留跨章节主线摘要。
- C43/C07 对话与 observation 历史会增长；应只压缩已完成且仍可由稳定 ID 回读的内容，保留最新纠正、错误、未完成状态和完整工具调用批次。

**需要按任务条件补信息的路径：**

- C25 已提供对应来源原句；文字层选取与真实模型还原质量仍需按任务评估，合同不要求所有候选 speech 都呈现。
- C15 检查历史范围时需区分区间起点角色事实和当前最新状态，并将规范名/别名及可支持判断的证据纳入模型上下文。
- C01/C16/C37/C39 的原文发现任务需保证范围召回；仅按最近资料、known names 或摘要选择，可能漏掉重现实体、别名或无对白行动。
- C45/C51/C52 以及漫画/视频素材上下文中，文件路径、图像 ID 和旧 prompt 不是图像像素。要求像素级一致、判断图片缺陷或跨帧比较时，必须确认实际 vision/image attachment 与文本上下文在该次调用都到达 provider。

若要把“候选最小集”升级为经过验证的生产决策，可先按产品合同决定是否修正上述输入缺口，再在代表性小、中、大项目上做配对消融：记录实际最终 provider prompt token、输出 schema/来源校验、人工盲评的事实覆盖/连续性/重复率和错误类型。减少 token 而导致引用错误、漏必要节拍或角色误判，不能算有效缩减。本次仅做源码静态核对，未运行模型、业务数据库或消融实验。

## 12. 字段清单外的调用

下面三类在字段清单中排除于 49 个聊天式业务入口之外，但会访问模型服务。列出它们以明确此次“逐项评估”的边界；图片/视频 provider 执行的是媒体生成合同，不是对话分析。

### C53 LLM 配置连通性探测

- **功能/时机/产物：** 用户测试某条模型配置时发送固定 `ping`；返回非空响应/usage，失败则显式报错。Sidecar profile 调 probe，普通 profile 调聊天 provider。
- **可用：** A 有已选 provider 配置及配置上下文上限；B 是项目/用户资料，但此任务不需读取。C 是探测响应。
- **实际输入：** 单条 user `ping`、temperature=0、max_tokens=min(512,configured output cap)。配置字段选择 provider/model/端点，不作为创作 prompt 发送。
- **结论：近似合理。** 对“能否连通且能否产出文本”是最小探测；它不验证 JSON 遵循、工具能力、长上下文或业务质量，不能把成功解释成这些能力均已验证。
- **候选最小集：** 固定短 ping 和受配置上限约束的输出预算已足够；项目正文、长 system prompt 或业务数据会改变探测目的。
- **证据：** [配置解析、调用分支和请求](../crates/host/a4n-runtime/src/actions/config_records/llm/connectivity.rs#L11)。

### C54 Chat Completions 图像生成 provider

- **功能/时机/产物：** 上游图像生成工作流提交一段最终图像 prompt，可附指定参考图，provider 返回解码后的图片 bytes。
- **可用：** A 有最终 prompt、宽高、模型和被选中的 reference_images(label,bytes)。B 是未绑定的项目/人物信息；provider 不会自行查询。C 是图像响应。
- **实际输入：** user 文本包含 prompt 和输出尺寸；有参考图时附 image_url/data URI parts 与标签顺序，wire 另传 `modalities=image,text`、size、n=1。
- **结论：近似合理，取决于上游 prompt 完整性。** 图像模型直接执行描述，输入已包含规格和明确选中的视觉参照。它不会自动知道章节剧情或 profile；若 prompt 缺了必须画出的事实，应在上游 prompt 构建阶段修正，不能指望 provider 从项目 ID 补齐。
- **候选最小集：** 可执行的最终图像 prompt、输出尺寸/模型控制和确实需要的一组参考图。避免无关参考图；若要求比对/继承角色外观，选中的参考图像素应真实传入。
- **证据：** [图像 user content 与参考图适配](../crates/image/a4n-image-providers/src/chat/request.rs#L1)、[图像 provider 发送请求并解码 bytes](../crates/image/a4n-image-providers/src/chat.rs#L40)。

### C55 Chat Completions 视频生成 provider

- **功能/时机/产物：** 上游视频工作流提交最终 prompt/负面 prompt、时长/画幅/帧率及所选媒体约束，provider 同步返回视频 bytes。
- **可用：** A 有最终 prompt、negative_prompt、输出参数及 media 中挑选的首帧/尾帧/参考图片来源。B 是未绑定的章节正文和项目资料；provider 不会自行读取。C 是生成视频。
- **实际输入：** user 文本包括 prompt 和尺寸/时长/fps；wire 另传 negative_prompt、reference_image_urls、image/last_frame 及视频模态控制。同步返回，无轮询 job。
- **结论：近似合理，需以上游 prompt 与媒体选择完整为条件。** provider 接口只执行当前请求，不负责把剧情/分镜/素材目录转成拍摄意图。若需要连续性，提供对应边界帧或参考图；仅传 ID 而没有 adapter 可访问的媒体位置不能建立视觉连续性。
- **候选最小集：** 已成型的 prompt/negative prompt、必要输出参数、确实影响连续性的首尾帧或参考图；不要附无关项目文本。检查上游引用解析已把 media 绑定到有效图片。
- **证据：** [视频媒体字段到 wire body 的映射](../crates/video/a4n-video-providers/src/chat/request.rs#L1)、[视频 provider 调用与响应处理](../crates/video/a4n-video-providers/src/chat.rs#L69)。
