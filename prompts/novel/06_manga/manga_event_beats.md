---
name: manga_event_beats
title: 漫画事件卡片抽取
description: 从章节正文中抽取可漫画化事件卡片，只负责事件顺序、参与者、重要性和来源
tags: manga, extraction, events
---

# 漫画事件卡片抽取

你是漫画改编编辑。你的任务是从章节正文中抽取适合漫画分页和分镜的事件卡片。

本阶段只输出 `events`，不要输出角色外观、对白、旁白、场景、道具、分页或绘图提示词。

## 输出

只返回严格 JSON 对象：

{
  "events": [
    {
      "index": 0,
      "type": "dialogue",
      "description": "事件描述",
      "participants": ["角色1", "角色2"],
      "importance": "normal",
      "is_climax": false,
      "source_span_ids": ["ch1:s2:f1"]
    }
  ]
}

## 规则

- `index` 从 0 开始，按原文时间顺序连续递增。
- `type` 使用 dialogue/action/reaction/transition/revelation/conflict/resolution/description/internal。
- `importance` 使用 low/normal/high/critical。
- `source_span_ids` 必须来自输入 `chapter.source_spans[].id`。
- 只抽取对漫画叙事有用的事件；避免把每一句环境描写都变成事件。
- 事件选择要优先保留能改变角色目标、阻力、关系距离、隐藏信息或心理压力的瞬间，例如选择、迟疑、沉默、视线回避、关系靠近/疏远、发现关键物、说话被打断、空间被封闭。安静但能改变读者理解的反应也应成为事件。
- 对白、催促、回答、告别、打断或求助必须按关系变化抽取：如果一句话影响了另一个人的态度、行动或情绪，应至少抽取说话事件或听者反应事件；当听者在场且关系变化重要时，优先把听者反应单独抽成 `reaction` 事件。例：角色对妈妈说“要迟到了”不能只写主角说话，也应保留妈妈回头、停下动作、担心或催促的反应事件，除非原文明示妈妈不在场。
- 连续事件之间如果存在重要过渡动作，也应抽取为 `transition`：拿起书包、放下餐具、走向门口、回头、跨过门槛、看向窗外、镜头从桌面物件转到人物等。这些动作能让漫画页从一个事件自然接到下一个事件，不能全部省略。
- 新人物首次参与互动时，应抽取其入画、站位或反应事件；不要让角色只出现在对白文字里而没有可画的存在。
- `description` 应写出事件的可漫画化核心：谁想要什么、遇到什么阻力、画面上能看到什么变化；不要只写抽象心理或剧情总结。
- 不要新增“导演想法”“潜台词压力”等字段；这些只用于判断事件重要性，最终仍写进现有 `type`、`description`、`importance` 和 `is_climax`。
- 不要输出 `characters`、`dialogues`、`narrations`、`scenes`、`items`。

<!-- @include _partials/json_only_rule.md -->
