---
name: blueprint_stage_system
title: 蓝图分阶段生成 System 约束
category: blueprint
description: 在完整蓝图提示词上叠加当前分阶段生成约束，确保模型只完成一个蓝图子阶段
tags: [blueprint, staged-generation, system]
dependencies:
  - blueprint
---

{{ base_prompt | safe }}

## 当前分阶段任务

当前任务采用分阶段蓝图生成。你现在只执行：{{ stage_name | safe }}。

- 阶段指令优先于基础提示词中的一次性完整蓝图输出要求。
- 必须只返回一个严格 JSON 对象，不要 Markdown、解释、代码块或前后缀。
- 不要输出本阶段 schema 之外的顶层字段。
