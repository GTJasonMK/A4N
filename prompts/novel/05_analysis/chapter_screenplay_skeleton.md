---
name: chapter_screenplay_skeleton
---

# 章节叙事骨架

只输出严格 JSON。根字段只能是 `schema_version`、`directing_statement`、`units`。
`schema_version` 固定为 `chapter_screenplay_skeleton_v1`。

`directing_statement` 的八个字段全部为非空 string：`central_dramatic_question`、`protagonist_throughline`、`audience_experience`、`viewpoint_strategy`、`visual_strategy`、`spatial_strategy`、`sound_strategy`、`rhythm_strategy`。

`units` 为 object[]。每项只能包含：`scene_identity` string、`sequence_index` 正整数、`narrative_unit_index` 正整数、`timeline_position` 正整数、`intercut_group_id` string|null、`editing_mode` string、`source_span_ids` string[]。

`narrative_unit_index` 与 `timeline_position` 都必须按影片实际呈现顺序从 1 连续编号到 units 数量，且全章不得重复或跳号。插叙、回忆、蒙太奇和平行动作仍按它们在成片中的出现顺序编号；其非线性时间语义使用 `editing_mode`、`sequence_index` 和 `intercut_group_id` 表达，不得通过重复 `timeline_position` 表达。同一场景连续拆成多个 unit 时，每个 unit 仍必须拥有不同的 `timeline_position`。

必须基于完整章节统一决定场景回切、时间线和跨场景穿插。相同物理场景或行动线回切时复用 `scene_identity`，但拆成不同 unit。`editing_mode` 只能是 `continuity`、`intercut`、`montage`、`flashback`、`parallel_action`、`sound_bridge`；`intercut` 和 `parallel_action` 必须设置非空 `intercut_group_id`。

本阶段禁止输出标题、场记、参与者、戏剧问题、状态、转场细节、表演事实或镜头。每个 unit 至少引用一个输入 source span，不得补写正文外事实。保持输出紧凑。
