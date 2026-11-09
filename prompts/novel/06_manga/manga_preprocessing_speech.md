---
name: manga_preprocessing_speech
---

# 漫画预处理：对白与旁白

根据正文及已经提取的事实索引，只输出 `speech`，结构为 `{"speech":{"dialogues":[],"narrations":[]}}`。字段合同以请求内 `output_schema` 为准。

- 不修改或重建事件和人物索引。所有 `source_event_id` 必须来自输入 `facts.source_events.events[].event_id`。
- 对白最多 24 条，单条不超过 30 字，仅保留推动剧情、揭示信息或塑造角色的内容。每项包含 `index`、`speaker`、`content`、`source_event_id`、`emotion`、`target`（无对象时为 null）、`is_internal`、`bubble_type`（normal 或 thought）。角色使用事实索引的稳定名称，不使用“叙述者”代替未具名主角。
- 旁白最多 8 条，单条不超过 40 字，仅保留无法由画面表达的时间或认知信息。每项包含 `index`、`content`、`source_event_id`、`narration_type`、`position`；不重复画面动作。
- 无合适内容时返回空数组，不编造对白。不得输出事实索引、场景、道具、综合、分页或改编节拍。

<!-- @include _partials/json_only_rule.md -->
