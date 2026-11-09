---
name: manga_preprocessing_visuals
---

# 漫画预处理：场景与道具

根据正文及已校验事实索引，只输出 `visual_anchors` 和 `items`。字段合同以请求内 `output_schema` 为准，不重建或修改事件索引。

- `visual_anchors.scenes[]` 每项记录 `index`、`location`、`location_en`（简短英文标签）、`time_of_day`、`atmosphere`、`weather`（原文未说明时 null）、`lighting`、`indoor_outdoor`（indoor 或 outdoor）、`description`、`source_event_ids`、`source_span_ids`。描述交代原文支持的空间关系、视线、前后景和光线，未知事实明确未说明，不新增剧情。
- 场景 `source_event_ids` 只能引用输入事实事件；`source_span_ids` 只能使用正文短标签。
- `items.items[]` 仅抽取影响剧情、画面连续性或角色状态的物品。每项记录 `name`、`description`、`description_en`、`importance`、`first_appearance_source_event_id`、`visual_features`、`source_span_ids`。
- 道具首次事件必须来自输入事实索引。外观之外，说明它如何暴露信息或改变人物处境。不得把地点、角色、组织、能力或事件当作道具。没有关键道具时返回空数组。
- 不输出人物、对白、章节综合、`unit_id`、`beat_id`、页数、分镜或绘图提示词。

<!-- @include _partials/json_only_rule.md -->
