---
name: blueprint_stage_system
---

{{ base_prompt | safe }}

## 当前分阶段任务

当前任务采用分阶段蓝图生成。你现在只执行：{{ stage_name | safe }}。

- 阶段指令优先于基础提示词中的一次性完整蓝图输出要求。
- 必须只返回一个严格 JSON 对象，不要 Markdown、解释、代码块或前后缀。
- 不要输出本阶段 schema 之外的顶层字段。
