---
name: blueprint_optimization_user
title: 蓝图优化 User 请求
category: blueprint
description: 将当前蓝图上下文和用户优化重点整理为严格 JSON 优化请求
tags: [blueprint, optimization, user-request]
dependencies:
  - blueprint_optimization
---

请根据以下 JSON 输入优化蓝图。只返回 output_contract 允许的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。

输入：
{{ payload | safe }}
