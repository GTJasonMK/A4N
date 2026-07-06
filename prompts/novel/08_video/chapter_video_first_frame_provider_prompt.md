---
name: chapter_video_first_frame_provider_prompt
title: 章节首帧串联 Provider 请求 Prompt
category: video
description: 将首帧串联模式的最终视频主提示词、动作补充和起步输入图语义渲染为实际提交给视频生成 provider 的文本
tags: [video, provider, prompt, first-frame]
dependencies:
  - chapter_video_first_frame_video_prompt
  - chapter_video_first_frame_prompts
---

{% if has_first_frame_image %}请根据以下导演提示生成一段连续视频；本次起步输入图片是视频第 0 秒画面，必须从这张图自然推进，不生成字幕、文字叠加、水印、拼贴、画中画或背景音乐。
{% else %}请根据以下导演提示生成一段连续视频；本段需要起步输入图片作为视频第 0 秒画面，当前请求未绑定该图片时仅可用于预览，真实生成前必须补齐；不生成字幕、文字叠加、水印、拼贴、画中画或背景音乐。
{% endif %}

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
