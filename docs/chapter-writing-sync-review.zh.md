# 章节正文分析与续写上下文审查

- 审查日期：2026-10-09
- 基线：`82cfcd08bc9e25491e6175fde320f8d2973416a9`
- 范围：正文保存/改写、章节分析产物写入、派生资料撤回、下一章提示上下文。
- 方式：基于基线静态审查；本分支实施修复并补充定向回归。没有运行模型或服务；数据库测试用例已编译，但因缺少独立测试库未执行。

## 结论

章节级写入总体有来源约束：自动章纲按项目和章节定位并校验正文快照；伏笔变更需要当前章节的原文证据；场景记录替换限定到当前 `chapter_id`；角色重同步会从被分析章节重算到档案已同步位置。未发现这些写入路径会直接把一个章节的内容写到另一个章节。

审查发现一项确认的数据一致性缺口和三项续写上下文风险，现均已在本分支修复。

## 确认问题

### CR-1（已修复）：正文来源失效后，AI 物体档案的章节范围和证据文本会残留

- 严重度：中
- 复现：第 5 章发现一个此前未出现的物体，档案写入 `first_seen_chapter=5`、`last_seen_chapter=5` 和本章证据；之后改写第 5 章并删掉该物体。
- 根因：正文清理会删掉该章的正文锚点及关联 `profile_evidence`，但物体档案的事实快照只包含别名、类型、描述、材质、用途和位置，没有 `first_seen_chapter`、`last_seen_chapter` 或 `evidence_json`。回滚事实时这些聚合字段不会重算。
- 影响：项目物体档案列表仍会显示旧的第 5 章范围和已失效原文；重新分析其他章节时，证据文本合并逻辑也可能继续保留这段旧证据。按章节组装续写上下文时，AI 生成的物体档案会因没有当前锚点证据而被过滤，所以目前不能断言它一定污染下一章提示词；档案页面及读取项目级物体档案的其他消费者仍会看到陈旧值。
- 建议：来源撤回后，根据剩余当前锚点重算 AI 档案的首次/最近出现章节和证据文本。若没有剩余正文来源，可保留身份，但应清空有效证据并标记为待复核；不要覆盖用户手工档案。
- 证据：`crates/chapter/a4n-chapter-content-core/src/cleanup.rs:27-58`、`crates/platform/a4n-db/src/text_anchor/evidence.rs:28-82`、`crates/platform/a4n-db/src/fact_feedback/profiles.rs:14-23,253-288`、`crates/platform/a4n-db/src/asset/profiles/upsert.rs:303-327`、`crates/platform/a4n-db/src/asset/profiles/read.rs:7-42`。
- 修复：删除章节锚点证据引用后，按剩余、正文哈希仍匹配的锚点重算 AI 档案的首次/最近出现章节与证据文本。没有剩余来源时保留档案内容、将章节范围清零并清空证据；物体/场景资料列表把无有效证据的范围显示为“出场章节待正文确认”。手工档案字段不参与重算；之后重新发现物体时，空证据档案的章节范围会由新来源重新建立。

## 上下文问题（已修复）

### R-1：作者预设的未来物体/场景会进入较早章节的续写上下文

- 复现：手工物体档案或场景档案的首次出现章节是 8，在生成第 3 章时，该档案没有第 1 至第 2 章的正文证据。
- 观察：`object_profiles_before_chapter` 和 `scene_profiles_before_chapter` 在缺少历史证据时，会把所有用户拥有的档案作为 `Authored` 返回，没有检查 `first_seen_chapter`。提示上下文会把物体状态/用途或场景描述写成“作者预设”，但不会附带首次出现章节，因此模型无法判断这些内容是否应在当前章出现。数据库测试目前只用首次出现章节为 1 的作者档案验证了这一行为。
- 语义决定：`first_seen_chapter` 是故事时序边界。角色同步已有同样的首次出现章节门禁，因此作者预设的物体/场景也不能在首次出现前进入正文生成上下文。
- 修复：没有更早正文证据时，只有 `first_seen_chapter <= target_chapter` 的用户档案作为作者预设注入上下文；首次出现章节晚于目标章的档案被排除。对曾由正文更新的物体/场景可变字段，另按目标章读取 `profile_facts_before_chapter` 投影，避免把后续章节状态带入较早章节；没有正文状态历史的手工设定仍作为作者全局设定使用。
- 证据：`crates/platform/a4n-db/src/project/chapter/context/profiles.rs:17-67,73-131`、`crates/host/a4n-orchestrator/src/chapter_generation/context_before/resolution.rs:31-33,75-78`、`crates/host/a4n-orchestrator/src/chapter_generation/context_before/object_profile.rs:8-48`、`crates/host/a4n-orchestrator/src/chapter_generation/context_before/scene_profile.rs:9-74`、`crates/platform/a4n-db/tests/chapter_context_profiles.rs:287-365`。

### R-2：前文章节没有结构化分析时，生成仍可继续

- 复现：导入或手写多章正文，但没有对前文章节运行“生成分析”；为下一章提供简报后启动正文生成。
- 观察：生成前的检查只拒绝前文章节中仍在运行或可恢复的章节分析工作流，没有检查每个前文章节是否有有效 `analysis_data`、章节摘要、角色/场景状态或分析索引。前文原文和 RAG 检索仍可提供部分上下文，因此请求不会必然失败。
- 影响：生成可以成功，但缺少结构化线索和角色/物体/场景状态；用户若以为系统已自动提炼前文，可能得到连续性较弱的结果。
- 修复：章节正文生成、请求预览及其共用的生成前置检查，都会拒绝仍有可恢复分析工作流的前文；对有正文但分析缺失或不符合当前章节分析 schema 的前文，返回具体章节号并要求先完成/重跑分析。没有正文的前文章节无需分析。
- 证据：`crates/host/a4n-orchestrator/src/chapter_generation/application/generation.rs:681-730,1099-1113`、`crates/host/a4n-orchestrator/src/chapter_generation/context_before/resolution.rs:35-110`。

### R-3：不同章节的生成任务可并行，后续章节可能读取正在生成的前章

- 复现：第 2 章和第 3 章在短时间内分别启动生成。任务唯一键按项目和章节编号区分；对于尚不存在正文行的新章，开始生成时没有章节 `generating` 状态，普通状态检查无法发现排队中的第 2 章任务。
- 影响：第 3 章可能拿到第 2 章改写前的旧正文与旧分析作为上下文。
- 修复：受理时检查前文；任务排队后再验证；资料组装完成后再验证一次。任务预约和数据库开始生成共用同一项目锁，按 reservation 创建顺序选出唯一可进入生成的章节任务，并原子设置现有章节的 `generating` 状态；尚无正文行的新章也由活动 reservation 纳入顺序。准备期间正文或状态变化时，现有来源指纹会拒绝保存旧上下文结果。
- 证据：`crates/host/a4n-orchestrator/src/context/generation/reservation.rs:254-285`、`crates/host/a4n-orchestrator/src/chapter_generation/application/generation.rs:42-80,681-730,749-805,927-1040`、`crates/platform/a4n-db/src/chapter_content/generation.rs:14-105`。

## 收尾优化

- 章节上下文原先按每个物体/场景档案单独查询事实历史。现改为按档案类型批量读取，并让单档案与批量路径共用同一历史读取函数，避免档案数量增加时产生逐条数据库往返。
- 检查本分支其他差异后，没有发现可安全删除的新增兼容分支、回退路径或未使用 API；审查中保留的现有恢复逻辑有实际调用方，不作为死代码清理。

## 已核对边界

- 回顾性章纲写入会校验来源章节快照，并按同一项目和章节编号查找；用户正式章纲没有自动来源标记时会被保留：`crates/platform/a4n-db/src/project/outline_records/provenance.rs:45-85,95-100,146-159`。
- 章节伏笔更新会验证正文来源版本、证据属于当前正文、setup 早于当前章节；不能用当前章节修改未来伏笔：`crates/platform/a4n-db/src/project/foreshadowing/sync.rs:28-37,157-182`。
- 场景同步在写入前验证项目、章节 ID 和章节号，并只删除/替换该章节的出场与转场记录：`crates/platform/a4n-db/src/scene_profile/sync/replacement.rs:17-35,80-107`。
- 角色档案同步使用 `AnalysisResyncFromTarget`，当目标章节早于 `last_synced_chapter` 时会从目标章重新计算到旧同步末尾：`crates/host/a4n-orchestrator/src/character/profile_sync/batch.rs:210-226`、`crates/host/a4n-orchestrator/src/character/sync_rules.rs:257-270`。
- 章节生成的 RAG 检索范围限定为目标章之前的章节，并限制可检索类型，不会从后续章节的 RAG 记录召回：`crates/host/a4n-orchestrator/src/chapter_generation/rag_context/preparation.rs:53-60`、`crates/host/a4n-orchestrator/src/chapter_generation/rag_context/rules.rs:43-69`。
- 正文变更会清除章节上的摘要、风格标签和结构化分析，并撤回后续依赖 checkpoint；后续章节的结构化状态需要重新分析/同步，不会在正文保存时自动重跑所有模型：`crates/platform/a4n-db/src/chapter_content.rs:30-45`、`crates/chapter/a4n-chapter-content-core/src/cleanup.rs:63-70`。
- 章节分析重跑时，若模型省略之前的伏笔回收结果，现有逻辑会保留该结果；这是有测试覆盖的保守策略，不是本次确认的缺陷：`crates/platform/a4n-db/tests/foreshadowing_sync.rs:92-150`。

## 修复验证

回归覆盖包括：撤回章节来源后 AI 物体档案范围/证据重算、用户档案不被覆盖、作者物体/场景的未来状态按目标章节投影、未来首次出现的档案不进入较早章节上下文、前文分析缺失/无效时阻止生成，以及前文章节生成状态和排队顺序阻止后续章越序启动。

- `cargo fmt --all -- --check`、`git diff --check`：通过。
- `cargo check -p a4n-db -p a4n-orchestrator --tests`：通过，包含全部新增后端回归测试的编译检查。
- `cargo test -p a4n-orchestrator --lib prior_chapter_analysis_tests`：5 项通过。
- `cargo test -p a4n-db --lib profile_merge_replaces_ai_range_after_source_evidence_was_removed`：1 项通过。
- `npm run test:run -- src/pages/novel-detail/shared.test.ts`：2 项通过；`npm run build`：通过。

数据库回归用例 `source_anchor_deletion_recomputes_ai_object_evidence_and_preserves_user_fields`、`chapter_context_uses_only_historical_evidence_and_occurrences` 和 `only_one_chapter_per_project_can_enter_generation` 已通过编译，但未运行：该工作树没有配置独立的 `A4N_TEST_DATABASE_URL`。因此锚点撤回、批量事实投影和项目级章节生成顺序的数据库运行时行为仍需在有隔离测试库的环境执行确认。
