---
name: character_portrait_generation_user
title: 角色三视图与拆解设定图生成 User 请求
category: protagonist
description: 将角色档案、项目上下文和设定图约束整理为人物三视图提示词生成请求
version: "1.0"
---

请根据以下 JSON 输入生成可直接用于图像生成模型的中文人物三视图与人物拆解设定图 prompt。只返回 prompt 文本，不要输出解释。

输入：
{{ payload_json | safe }}
