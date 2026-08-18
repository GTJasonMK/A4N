---
name: blueprint_stage_user
---

请根据以下 JSON 输入执行当前蓝图生成阶段。只返回 current_stage.output_schema 对应的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。

{% if stage_key == "story_core" %}
当前阶段指令：请生成故事核心定位。只输出 JSON 对象，字段只能包含 title、target_audience、genre、style、tone、one_sentence_summary、full_synopsis。不要输出 world_setting、characters、relationships、chapter_outline。
{% elif stage_key == "world" %}
当前阶段指令：请基于故事核心生成完整 world_setting。只输出 JSON 对象，顶层只能包含 world_setting。world_setting 必须包含 premise、core_rules、technology_or_magic、key_locations、factions、history_timeline、culture_and_society、conflict_sources、mysteries_and_foreshadowing、genre_promises；数组字段至少 1 项。technology_or_magic 必须包含 source、rules、limits、costs；culture_and_society 必须包含 values、class_structure、daily_life；每个 key_locations[] 必须包含 name、description、story_function；每个 factions[] 必须包含 name、description、agenda、representative_characters，且 representative_characters 必须是字符串数组；每个 history_timeline[] 必须包含 period、event、impact；每个 conflict_sources[] 必须包含 name、description、stakes；每个 mysteries_and_foreshadowing[] 必须包含 mystery、early_clues、payoff_direction，且 early_clues 必须是字符串数组。
{% elif stage_key == "characters" %}
当前阶段指令：请基于故事核心和世界观生成主要角色。只输出 JSON 对象，顶层只能包含 characters。至少 3 个角色；每个角色必须使用 name、character_type、gender、age、appearance、personality、background、motivation、arc_summary、relationship_to_protagonist、arc_stages；arc_stages 必含 start、core_wound、desire、need、secret、voice_style、turning_point、end。
{% elif stage_key == "relationships" %}
当前阶段指令：请基于故事核心、世界观和角色档案生成角色关系网络。只输出 JSON 对象，顶层只能包含 relationships。至少 1 条重要关系；每条关系必须使用 relationship_type、character_a、character_b、description、strength、evolution；strength 为 1 到 10 的整数；evolution 必含 start、tension、turning_points、hidden_truth、story_function、end。
{% endif %}

输入：
{{ payload | safe }}
