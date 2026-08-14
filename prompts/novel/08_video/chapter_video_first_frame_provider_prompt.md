---
name: chapter_video_first_frame_provider_prompt
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
