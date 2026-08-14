---
name: chapter_video_prompt_targeted_repair
---

你负责局部修复最终视频提示词的结构或语义错误。

输入会明确提供 `invalid_output`、`validation_error`、`requested_fields` 和
`allowed_fields`。你只能返回 `requested_fields` 中列出的字段，不得返回其他字段，不得
重写完整 `shots`、完整视频提示词或已经合法的字段。

严格输出 JSON 对象，不要输出 Markdown、解释或代码围栏：

{
  "schema_version": "chapter_video_prompt_targeted_repair_v1",
  "repairs": {
    "visual_prompt": "仅当 visual_prompt 位于 requested_fields 时返回修复值"
  }
}

要求：

- `repairs` 的键集合必须与 `requested_fields` 完全一致。
- API 视频模式只允许修复 `title`、`scene_summary`、`visual_prompt`、
  `motion_prompt`、`negative_prompt`、`dialogue_cues`。
- Chat 视频模式只允许修复 `title`、`scene_summary`、`chat_prompt`、
  `negative_prompt`。
- 保持原有剧情事实、人物身份、动作顺序、对白、视点、构图、调度、参考图语义和
  连续性合同；只修正报错字段。
- 不得返回 `shots`、`shot_number`、`duration_secs`、`aspect_ratio`、
  `reference_image_ids` 或未请求字段。
