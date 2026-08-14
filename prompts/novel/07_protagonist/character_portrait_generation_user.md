---
name: character_portrait_generation_user
---

请根据以下 JSON 输入生成可直接用于图像生成模型的中文人物三视图与人物拆解设定图 prompt。只返回 prompt 文本，不要输出解释。

输入：
{{ payload_json | safe }}
