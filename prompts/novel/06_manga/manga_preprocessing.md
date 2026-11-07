---
name: manga_preprocessing
---

# 漫画改编前预处理

你负责把原文章节中的事实资料整理成漫画改编前预处理包。结果会提供给漫画改编计划和分页规划。

只使用原文和可信的角色素材参考，不生成媒介改编节拍、页数或具体分页内容，不输出画格分配或绘图提示词。最终页数由漫画改编计划确定。

输出严格 JSON：

```json
{
  "schema_version": "manga_preprocessing_v3",
  "source_events": {
    "events": [
      {
        "event_id": "event-1",
        "type": "action",
        "description": "可被画面表现的事件",
        "participants": ["角色名"],
        "importance": "critical",
        "source_span_ids": ["#0"]
      }
    ]
  },
  "characters": {
    "角色名": {
      "name": "角色名",
      "appearance": "发型、服装、体型和年龄特征",
      "personality": "能指导表情、姿态和视线的稳定倾向",
      "role": "protagonist",
      "gender": "unknown",
      "age_description": "青年",
      "source_span_ids": ["#0"]
    }
  },
  "speech": {
    "dialogues": [{
      "index": 0,
      "speaker": "角色名",
      "content": "需要保留的短对白",
      "source_event_id": "event-1",
      "emotion": "迟疑",
      "target": null,
      "is_internal": false,
      "bubble_type": "normal"
    }],
    "narrations": [{
      "index": 0,
      "content": "画面无法表达的时间或认知信息",
      "source_event_id": "event-1",
      "narration_type": "time",
      "position": "top"
    }]
  },
  "visual_anchors": {
    "scenes": [{
      "index": 0,
      "location": "地点",
      "location_en": "location label",
      "time_of_day": "night",
      "atmosphere": "关系距离和空间压力",
      "weather": null,
      "lighting": "能表现角色处境的光线",
      "indoor_outdoor": "indoor",
      "description": "包含遮挡、前后景、视线和空间关系的视觉描述",
      "source_event_ids": ["event-1"],
      "source_span_ids": ["#0"]
    }]
  },
  "items": {
    "items": [{
      "name": "关键物品",
      "description": "中文描述",
      "description_en": "绘图补充描述",
      "importance": "prop",
      "first_appearance_source_event_id": "event-1",
      "visual_features": "外观及其推动目标、阻力、关系或读者理解的画面功能",
      "source_span_ids": ["#0"]
    }]
  },
  "comic_synthesis": {
    "chapter_summary": "2-3句话概括漫画化后的叙事重点",
    "mood_progression": ["平静观察", "疑点逼近", "关系失衡", "真相暴露", "余波沉默"]
  }
}
```

## 规则

- 至少为每个影响剧情、人物状态或空间关系的原文事实建立一个 source event；不得合并成改编节拍。
- `source_events.events[].event_id` 唯一，`type`、`description` 非空，`participants` 为角色名数组，`source_span_ids` 至少包含一个原文来源。
- `characters` 只收录本章正文实际出现的角色。“叙述者”只表示文字叙述身份，不是角色名；不得仅因第一人称叙述创建 `characters.叙述者`。第一人称代词不直接作为角色名；只有上下文能确认时才沿用立绘规范名。若第一人称视角对应的人物实际出现在剧情或画面中，使用稳定身份键 `未具名主角`，外观仅记录原文明示的事实；纯画外叙述者不作为角色。不得补造身份或外观。立绘只辅助身份和外观，不证明角色在本章出现。
- `characters.*.role` 使用 `protagonist`、`antagonist`、`supporting`、`minor` 或 `background`；`gender` 使用 `male`、`female` 或 `unknown`。外观和性格应能指导稳定绘制与表演。
- 对白最多 24 条，旁白最多 8 条；只保留推动剧情、揭示信息、塑造角色或无法由画面表达的内容。对白单条不超过 30 字，旁白单条不超过 40 字。
- `speech.dialogues[]` 和 `speech.narrations[]` 通过 `source_event_id` 关联原文事件，不使用 beat 引用。对白情绪应反映现场压力；旁白不要重复画面已经表达的动作。
- `visual_anchors.scenes[]` 使用 `source_event_ids` 关联原文事件。地点、光线、遮挡、前后景、视线和空间距离应支持读者理解人物处境；`location_en` 使用简短英文标签。
- 只抽取影响剧情、画面连续性或角色状态的物品。物品外观之外，`visual_features` 还要说明它如何成为焦点、暴露信息、改变关系距离或制造阻力；不要把地点、角色、组织、能力或事件当作物品。
- `items.items[]` 通过 `first_appearance_source_event_id` 关联最早出现的原文事件。
- `comic_synthesis.chapter_summary` 用 2-3 句话概括主角目标、主要阻力、关系或隐藏信息的关键变化，以及最需要交付的情绪转折，不要复述事件流水账。
- `comic_synthesis.mood_progression` 输出 3-6 个递进阶段，体现压力变化和关键转折，不只罗列气氛词。
- `comic_synthesis` 只概括章节叙事重点和情绪递进，不决定篇幅或页数。
- 章节综合只能总结上面的原文事实，不得补充新剧情。
- 所有 `source_span_ids` 必须来自输入正文短标签；所有事件 ID 引用必须指向本次 `source_events.events[]` 中的 ID。不要输出 `beat_ref`、`unit_id`、`beat_id`、页数、改编取舍或分页结果。

<!-- @include _partials/json_only_rule.md -->
