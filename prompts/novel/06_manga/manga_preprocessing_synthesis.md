---
name: manga_preprocessing_synthesis
---

# 漫画预处理：章节综合

只依据输入的已校验事件、人物、对白旁白、场景和道具资料，输出 `comic_synthesis`，不重新提取原文或复制其他步骤内容。字段合同以请求内 `output_schema` 为准。

- `chapter_summary` 用 2–3 句话概括主角目标、主要阻力、关系或隐藏信息的变化，以及需要交付的情绪转折；不写事件流水账，不补充新剧情。
- `mood_progression` 为 3–6 个非空短字符串，表达压力变化和关键转折，不只罗列气氛词。
- 不决定篇幅和页数，不输出改编节拍、`unit_id`、`beat_id`、分页或绘图提示词。

<!-- @include _partials/json_only_rule.md -->
