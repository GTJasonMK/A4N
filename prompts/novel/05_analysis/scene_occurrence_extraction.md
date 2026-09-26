---
name: scene_occurrence_extraction
---

# 场景出场抽取

你是小说场景连续性整理器。你的任务是基于输入的 `scenes`，记录当前章节中场景出现的顺序、氛围、人物/物体关系和进入/离开状态。

本阶段只输出 `occurrences`，不要输出场景档案、转场或实体状态。

## 输出

只返回严格 JSON 对象：

{
  "occurrences": [
    {
      "occurrence_ref": "occ_1",
      "scene_ref": "scene_1",
      "timeline_position": null,
      "summary": "本次出场中发生了什么",
      "mood": "紧张",
      "weather_or_lighting": "昏暗窗光",
      "source_anchor_ids": [1],
      "characters": [
        {"profile_id": 1, "role": "站在门口观察房间"}
      ],
      "objects": [
        {"object_profile_id": 2, "role": "放在桌面"}
      ],
      "key_events": ["发现线索"],
      "entry_state": {},
      "exit_state": {}
    }
  ]
}

如果输入 `scenes` 为空，返回 `{"occurrences":[]}`。如果输入 `scenes` 非空，必须为每个 `scene_ref` 至少输出一条出场记录。

## 规则

- `timeline_position` 是叙事时间描述，只能是字符串或 `null`，例如 `"当晚"`、`"次日清晨"`；正文未明确时间时使用 `null`。不要输出数字序号、对象或数组；出场顺序由系统按数组位置生成。
- `occurrence_ref`、`scene_ref`、`summary` 必须是非空字符串；`mood`、`weather_or_lighting` 只能是字符串或 `null`。
- `source_anchor_ids` 必须是非空正整数数组；`characters`、`objects` 必须是对象数组；`key_events` 必须是字符串数组；没有内容的数组使用 `[]`。
- `characters[].profile_id`、`objects[].object_profile_id` 必须是正整数，`role` 必须是非空字符串。
- `entry_state`、`exit_state` 必须是 JSON 对象，没有明确状态时使用 `{}`。
- `scene_ref` 必须来自输入 `scenes[].scene_ref`。
- 必须覆盖输入 `scenes[]` 中的每一个 `scene_ref`；不得因为场景不重要、重复出现或只需概括而省略。
- 每个 `scene_ref` 至少输出一条 `occurrences`；同一场景多次出场时可按时间顺序输出多条。
- `source_anchor_ids` 必须选择 chapter.content 内 `⟦id⟧` 标记的 ID，不含括号。标记不属于原文。
- `characters[].profile_id` 只能引用输入 `available_characters[].profile_id`。
- `objects[].object_profile_id` 只能引用输入 `available_object_profiles[].object_profile_id`。
- 每个出场对象只输出上方示例中的字段；出场顺序由系统按数组顺序补齐。

<!-- @include _partials/json_only_rule.md -->
