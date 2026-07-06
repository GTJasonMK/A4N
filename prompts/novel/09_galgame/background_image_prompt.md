---
name: galgame_background_image_prompt
title: Galgame 背景图提示词
description: 将背景规划和项目美术约束渲染为图像生成提示词
tags: galgame, background, image-prompt
---

视觉小说 Galgame 背景图，16:9 横构图，可直接作为对话场景背景使用，空镜或弱人物背景，无清晰主角立绘，无 UI、无文字、无水印、无字幕、无对白框。
场景标题：{{ scene.title }}。
{% if scene.location %}地点：{{ scene.location }}。{% endif %}
{% if scene.time_of_day %}时间：{{ scene.time_of_day }}。{% endif %}
{% if scene.background_note %}背景规划：{{ scene.background_note }}。{% endif %}
画面要求：环境信息清晰可读，空间层次明确，光线与场景情绪一致，透视稳定，细节精致但不抢占角色立绘区域。
风格约束：保持视觉小说美术质感，适合作为 Galgame 背景资产；不要加入输入没有依据的品牌、文字标识、现代设备或额外剧情信息。
