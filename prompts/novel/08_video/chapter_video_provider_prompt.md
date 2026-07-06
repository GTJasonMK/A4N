---
name: chapter_video_provider_prompt
title: 章节视频 Provider 请求 Prompt
category: video
description: 将最终视频主提示词、动作补充和输入图片语义渲染为实际提交给视频生成 provider 的文本
tags: [video, provider, prompt]
dependencies:
  - chapter_video_prompt
  - chapter_video_frame_prompts
---

请根据以下导演提示生成一段连续视频；严格按主提示词执行，不生成字幕、文字叠加、水印、拼贴、画中画或背景音乐。

{% if subject_asset_bindings %}
【输入图片顺序】
{{ subject_asset_bindings | safe }}
{% endif %}

【视频生成主提示词】
{{ visual_prompt | safe }}

{% if motion_prompt %}

【动作与运镜补充】
{{ motion_prompt | safe }}
{% endif %}
