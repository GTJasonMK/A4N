---
name: chapter_media_adaptation_plan
---

你负责把原作章节改编为指定媒介的叙事计划。输入中的 target 为 manga 或 galgame。

以当前 `chapter.content` 中带短标签的原文为依据，不提供完整 `source_spans` 记录。`video_reference` 是可选的已冻结参考，可以借鉴场景取舍、动作、对白意图和人物关系；不要把镜头时长、机位或声音要求当成漫画或游戏的硬约束。参考使用了其他原作版本时，不得把它的原文引用当作当前原文证据。

当 `target` 为 `manga` 时，`manga_preprocessing` 是当前章节的原文预处理资料。它只用于整理角色、事件、对白、场景和物品事实；改编计划仍必须自己决定媒介化的叙事单元与节拍，不得照抄预处理为分页结果。

来源字段只能原样返回正文中出现的短标签，不能返回内部长 ID。

输出严格 JSON，不含 Markdown。顶层必须且只能包含：
- schema_version：固定为 "adaptation_plan_v1"。
- target：与输入完全相同。
- 当 `target` 为 `manga` 时，必须包含 `page_count`，取值为 1 到 100 的整数；当 `target` 为 `galgame` 时不得包含该字段。
- intention：本章改编目标，非空字符串。
- decisions：改编取舍数组，每项包含 description、reason、source_span_ids。
- units：非空叙事单元数组。

每个 unit 必须且只能包含：
- id：本计划内唯一、稳定的字符串。更新 previous_plan 时，同一单元保留原 id；重排不改 id。
- origin：source（直接来自原文）、adapted（重组原作）、added（新增改编）、historical（仅用于既有历史迁移，生成时不要使用）。
- title：标题。
- location：地点字符串或 null。
- participants：角色名称数组，沿用原作名称。
- dramatic_question、objective、obstacle：戏剧问题、目标、阻力。
- entry_state、exit_state：可观察的入场与离场状态。
- source_span_ids：当前正文中实际出现的短来源标签；source、adapted 类型至少一项，added 类型必须为空。单元的引用必须覆盖其所有节拍引用。
- beats：非空节拍数组。

每个 beat 必须且只能包含：
- id：单元内唯一、稳定的字符串，更新时保留既有身份。
- kind：action、dialogue、reaction、revelation 或 transition。
- is_climax：是否为本章高潮节拍，布尔值。
- origin：source、adapted 或 added。
- importance：required、important 或 supporting。
- action：具体动作或信息事件。
- reaction：反应字符串或 null。
- dialogue_intent：对白意图字符串或 null。
- entry_state、exit_state：动作前后状态。
- source_span_ids：当前正文的短原文来源标签；规则与 unit 相同。

当 `target` 为 `manga` 时，改编计划必须根据叙事单元、required 节拍、转折和阅读节奏决定本章最终页数。`page_count` 是后续分页规划必须遵守的固定数量，不是粗略估算。
Galgame 计划突出人物目标、对话推进和适合玩家参与的矛盾，具体选项与分支留给后续阶段。
叙事单元不是摄影镜头；一个单元可包含多个动作节拍。不要为每个原文段落机械建立一个单元。
