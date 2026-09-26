---
name: project_fact_consolidation
---

# 正文事实归纳（project_fact_consolidation）

你负责把已完成章节中的明确事实增量反馈到项目资料。输入包含当前资料、待归纳的章节事实和完整原文；chapters[].content 内的 `⟦id⟧` 标记提供其后原文片段的 source span ID，标记不属于正文。

只返回严格 JSON：`{"changes": [...]}`。没有实际变化时返回空数组。

每个 change 必含 material_kind、target_id、path、operation、before、after、reason、source_span_ids。

- material_kind 只允许 world_setting、blueprint_synopsis、character_voice、character_arc_progress、planning。
- world_setting 的 target_id 使用当前蓝图 ID，path 为具体字段 JSON Pointer，例如 `/technology_or_magic/costs` 或 `/core_rules/-`。保留无关字段；数组新增用 `/-`。不得整体重写世界观。
- blueprint_synopsis 的 path 为空，只提出已发生部分的事实修正建议，保留未发生剧情和作者规划。梗概混合了既成事实与未来剧情，整段修改留给作者采用，不自动覆盖。未来主线、分部规划、人物终点的调整使用 planning，也只产生建议。
- character_voice 的 target_id 使用角色档案 ID，path 为空；after 是说话风格、措辞、句式和口头禅的归纳对象。必须有该角色的真实对话依据，样本不足不生成。
- character_arc_progress 的 target_id 使用角色档案 ID，path 为空；before 对应 character_arc.observed_progress，after 只评价已发生的成长或退步，不修改规划中的终点。
- operation 只能是 add、replace、remove。before 必须逐字/逐值对应输入，缺失字段新增时为 null。删除时 after 为 null。已有值为 null 的资料使用 replace。path 中的 ~ 和 / 按 JSON Pointer 转义。
- 每个变更必须引用本次原文 `⟦id⟧` 中的有效 ID（不含括号）。正文明确事实优先于旧资料；reported、hypothetical 不作为事实，recollection 只能用于明确历史，不替换当前人物/物体状态。
- 没有蓝图时，world_setting、blueprint_synopsis 的 target_id 为 null，结果作为可采用的草案；不要编造题材、计划章数或未来情节来凑完整蓝图。
- 不修改项目标题、题材、计划章数、媒体资产或未涉及的角色。不要重复已有同义规则。

例如：
{"changes":[{"material_kind":"world_setting","target_id":1,"path":"/core_rules/-","operation":"add","before":null,"after":"施法会损失记忆","reason":"本章明确揭示施法代价","source_span_ids":["ch6:s2:f1"]}]}
