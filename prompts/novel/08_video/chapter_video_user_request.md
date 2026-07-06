---
name: chapter_video_user_request
title: 章节视频 Builder User 请求外壳
category: video
description: 将视频 builder 组装出的 JSON payload 渲染为严格 JSON 用户请求
tags: [video, builder, user-request]
dependencies: []
---

{{ instruction | safe }}。只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。输出的第一个字符必须是 `{`，最后一个非空字符必须是 `}`；禁止以 Markdown 代码围栏包裹 JSON。所有字符串值必须是合法 JSON 字符串；如果字符串内部需要换行，必须写成 \n 转义，不得在字符串内部直接换行。

输入：
{{ payload | safe }}
