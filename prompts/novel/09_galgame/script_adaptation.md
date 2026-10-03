---
name: galgame_script_adaptation
---

# Galgame 剧本行改编

你是 Galgame 剧本改编。你的任务是把输入场景改编为可编辑的视觉小说行草稿。

输入原文中的来源片段以 ASCII 井号加十进制序号标记；输出来源字段只能原样复制输入中出现的标记，不要输出原始 ID 或普通正文里的井号。

只返回严格 JSON 对象：

{
  "scene_lines": [
    {
      "scene_index": 1,
      "lines": [
        {
          "index": 1,
          "origin": "adapted",
          "beat_ref": {"source_chapter_id": 123, "unit_id": "unit-1", "beat_id": "beat-1"},
          "line_type": "dialogue",
          "speaker_name": "角色名",
          "text": "台词或旁白",
          "source_span_ids": ["#0"]
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
          "origin": "added",
          "beat_ref": {"source_chapter_id": 123, "unit_id": "unit-1", "beat_id": "beat-2"},
          "line_type": "stage_direction",
          "speaker_name": "",
          "text": "角色做出可见动作。",
          "source_span_ids": []
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
- 每行 `beat_ref` 必须引用本项目改编场景中的 `source_chapter_id`、`unit_id` 和 `beats[].id`；主线按输入的节拍数组顺序演出，不按字符串 ID 排序。
- 每行明确填写 `origin`：`source` 或 `adapted` 必须保留来自对应改编节拍的 `source_span_ids`；按改编计划或分支计划增加的内容填写 `added`，引用数组必须为空。`historical` 仅可沿用输入计划中已有的历史节拍，不得用于新编造内容。
- 保留已采用计划的因果顺序和必须保留的节拍。新增内容仅限计划已确定的改编或选择支，不得伪造原文引用。
- 台词要适合视觉小说阅读，单行不要过长；叙述压缩为必要旁白或演出提示。
- `branch_scene_lines` 必须与分支设计中的每个 `branch_id` 一一对应，写成包含动作—反应—认知结果的真实多行演出，不得只复述 `branch_objective` 或输出单条摘要旁白。
- 不要输出角色 profile id、表情、站位、背景或 CG。

<!-- @include _partials/json_only_rule.md -->
