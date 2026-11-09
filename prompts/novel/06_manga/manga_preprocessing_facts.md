---
name: manga_preprocessing_facts
---

# 漫画预处理：原文事件与人物

只完成原文事实提取，输出 `source_events` 和 `characters`，不输出完整预处理包或其他步骤字段。最终严格 JSON 的必填字段、类型和来源约束以请求内 `output_schema` 为准。

- `source_events.events[]` 为每个影响剧情、人物状态或空间关系的原文事实建立唯一 `event_id`，不要压缩成改编节拍。每项包含 `event_id`、`type`、`description`、`participants`、`source_span_ids`，可附 `importance`。
- `participants` 是角色名数组，来源只能使用输入正文短标签。描述保留关键事实，避免原文大段复述。不能新增剧情。
- `characters` 是以稳定角色名为键的对象，只收录本章实际出现的人物。每项记录 `name`、`appearance`、`personality`、`role`、`gender`、`age_description`、`source_span_ids`。未知外观、年龄和性格明确写“原文未说明”，不补造。
- `role` 使用 `protagonist`、`antagonist`、`supporting`、`minor` 或 `background`；`gender` 使用 `male`、`female` 或 `unknown`。
- 角色参考只辅助身份和外观，不证明角色在本章出现。“叙述者”不是角色名；实际出镜但无法具名的第一人称主角使用稳定键 `未具名主角`；纯画外叙述者不作为角色。
- 不输出对白、场景、道具、章节综合、`unit_id`、`beat_id`、页数、分页或绘图提示词。

<!-- @include _partials/json_only_rule.md -->
