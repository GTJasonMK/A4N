---
name: manga_prompt_optimization_user
---

请根据以下 JSON 输入优化一个已有漫画整页绘图提示词。只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。所有字符串值必须是合法 JSON 字符串；如果字符串内部需要换行，必须写成 \n 转义，不得在字符串内部直接换行。

输入：
{{ payload | safe }}
