---
name: blueprint_optimization_user
---

请根据以下 JSON 输入中的灵感对话、当前蓝图和优化重点优化蓝图。只返回 output_contract 允许的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。

输入：
{{ payload | safe }}
