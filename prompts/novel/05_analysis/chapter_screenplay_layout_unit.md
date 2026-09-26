---
name: chapter_screenplay_layout_unit
---

# 叙事单元布局细化

只处理当前任务指定的一个 `narrative_unit_index` 和 `locked_visual_unit`，不得修改稳定上下文中的骨架字段。只输出严格 JSON，根字段必须且只能包含：`schema_version`、`narrative_unit_index`、`source_span_ids`、`transition_contracts`、`title`、`slugline`、`participants`、`dramatic_question`、`scene_objective`、`scene_obstacle`、`entry_state`、`exit_state`。

`schema_version` 固定为 `screenplay_layout_unit_detail_v2`。`narrative_unit_index` 必须等于当前任务值。`participants` 和 `source_span_ids` 为 string[]。其余文本字段为非空 string。

`source_span_ids` 只选择当前导演单元实际涉及的原文 `⟦id⟧` 标记（不含括号），必须去重，不要复制全章候选 ID。后续改编只会收到这些引用所在的完整段落，因此应包含理解当前动作、人物状态和对白所必需的来源；不得引入其他单元或后续情节。仅依据用户导演新增内容时允许 `[]`，不要捏造正文依据。

`transition_contracts` 为 object[]；没有转场时输出 `[]`。每项必须包含：`to_narrative_unit_index` 正整数、`transition_type` 枚举、`preserved_action` string|null、`preserved_sound` string|null、`preserved_knowledge` string[]、`required_cut_point` string|null。`transition_type` 只能是 `continuous`、`hard_cut`、`match_cut`、`whip_pan`、`action_wipe`、`sound_bridge`、`dissolve`、`fade`、`focus_transition`、`morph`。数组即使只有一项也必须保留 `[]`。

转场不得指向自身或骨架外 unit。只依据当前 unit 的 source spans、全章骨架和完整章节上下文细化，不得输出表演事实、景别、机位、摄影运动、blocking 或 shots。
