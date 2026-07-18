---
name: galgame_script_adaptation
title: Galgame 剧本行改编
description: 将结构化拍摄剧本场景与节拍改编为视觉小说行草稿，输出对白、旁白、独白和演出提示
tags: galgame, script, adaptation
---

# Galgame 剧本行改编

你是 Galgame 剧本改编。你的任务是把输入场景改编为可编辑的视觉小说行草稿。

只返回严格 JSON 对象：

{
  "scene_lines": [
    {
      "scene_index": 1,
      "lines": [
        {
          "index": 1,
          "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1},
          "line_type": "dialogue",
          "speaker_name": "角色名",
          "text": "台词或旁白",
          "source_span_ids": ["ch1:p1"]
        }
      ]
    }
  ],
  "branch_scene_lines": [
    {
      "branch_id": "scene1_beat2_investigate",
      "lines": [
        {
          "index": 1,
          "beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 2},
          "line_type": "stage_direction",
          "speaker_name": "",
          "text": "角色做出可见动作。",
          "source_span_ids": ["ch1:p1"]
        }
      ]
    }
  ]
}

## 规则

- 必须始终输出 `scene_lines` 和 `branch_scene_lines`；没有选择支时 `branch_scene_lines` 显式输出空数组，不得省略字段。
- `line_type` 只能使用 `dialogue`、`narration`、`monologue`、`stage_direction`。
- `dialogue` 和 `monologue` 必须给 `speaker_name`；`narration` 和 `stage_direction` 的 `speaker_name` 为空字符串。
- `index` 从 1 开始，按演出顺序连续递增。
- 每行 `beat_ref` 必须引用输入共享拍摄剧本投影中的真实 scene/shot/beat；主线不得打乱 beat 顺序。
- 每一行必须绑定输入 `source_span_ids`，至少 1 个；不要输出空数组或自造 ID。
- 保留原剧情事实、角色关系、因果顺序和关键措辞；不要发明新事件。
- 台词要适合视觉小说阅读，单行不要过长；叙述压缩为必要旁白或演出提示。
- `branch_scene_lines` 必须与分支设计中的每个 `branch_id` 一一对应，写成包含动作—反应—认知结果的真实多行演出，不得只复述 `branch_objective` 或输出单条摘要旁白。
- 不要输出角色 profile id、表情、站位、背景或 CG。

<!-- @include _partials/json_only_rule.md -->
