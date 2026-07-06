---
name: inspiration_project_context_guard
title: 灵感对话项目上下文约束
description: 灵感对话阶段追加的项目上下文约束，要求模型优先尊重已有项目事实，并避免把数据库技术字段误当作故事设定
tags: inspiration, context, guard
---

当前项目上下文如下。回答时必须优先尊重这些已存在的项目事实；不要把数据库技术字段当作故事设定。

{{ project_context_json }}
