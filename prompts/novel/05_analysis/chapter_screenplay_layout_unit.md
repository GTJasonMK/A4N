---
name: chapter_screenplay_layout_unit
---

# 叙事单元布局细化

只处理当前任务指定的一个 `narrative_unit_index`，不得修改稳定上下文中的骨架字段。只输出严格 JSON，根字段必须且只能包含：`schema_version`、`narrative_unit_index`、`transition_contracts`、`title`、`slugline`、`participants`、`dramatic_question`、`scene_objective`、`scene_obstacle`、`entry_state`、`exit_state`。

`schema_version` 固定为 `screenplay_layout_unit_detail_v1`。`narrative_unit_index` 必须等于当前任务值。`participants` 为 string[]。其余文本字段为非空 string。

`transition_contracts` 为 object[]；没有转场时输出 `[]`。每项必须包含：`to_narrative_unit_index` 正整数、`transition_type` 枚举、`preserved_action` string|null、`preserved_sound` string|null、`preserved_knowledge` string[]、`required_cut_point` string|null。`transition_type` 只能是 `continuous`、`hard_cut`、`match_cut`、`whip_pan`、`action_wipe`、`sound_bridge`、`dissolve`、`fade`、`focus_transition`、`morph`。数组即使只有一项也必须保留 `[]`。

转场不得指向自身或骨架外 unit。只依据当前 unit 的 source spans、全章骨架和完整章节上下文细化，不得输出表演事实、景别、机位、摄影运动、blocking 或 shots。
