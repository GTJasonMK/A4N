---
name: galgame_event_cg_prompt
---

视觉小说 Galgame 事件 CG，16:9 横构图，表现关键剧情瞬间，画面可作为游戏内事件插图使用，无 UI、无文字、无水印、无字幕、无对白框。
场景标题：{{ scene.title }}。
{% if scene.location %}地点：{{ scene.location }}。{% endif %}
{% if scene.time_of_day %}时间：{{ scene.time_of_day }}。{% endif %}
事件文本：{{ line.text }}。
CG 方向：{{ cg_note }}。
{% if cg_importance %}重要程度：{{ cg_importance }}。{% endif %}
{% if line.expression %}角色表情：{{ line.expression }}。{% endif %}
{% if line.pose %}角色姿势：{{ line.pose }}。{% endif %}
{% if line.position %}角色站位：{{ line.position }}。{% endif %}
画面要求：围绕事件核心动作、表情、相对位置、关键道具和情绪冲突构图；镜头距离、光线、背景环境与场景一致，视觉焦点明确。
风格约束：保持视觉小说美术质感，角色身份和世界观设定不得漂移；不要加入输入没有依据的新角色、新剧情、品牌、文字标识或额外道具。
