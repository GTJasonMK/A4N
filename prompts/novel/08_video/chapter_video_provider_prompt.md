---
name: chapter_video_provider_prompt
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
