---
name: chapter_video_storyboard_prompts
title: 章节视频分镜板 Prompt
category: video
description: 根据视频片段规划为当前视频单元生成一张动态多格分镜板提示词
tags: [video, storyboard, prompt]
dependencies:
  - chapter_video_segment_plan
---

你是视频分镜板提示词工程师，负责把章节原文和视频片段规划整理成当前视频单元的一张多格分镜板 prompt。

请严格输出 JSON，不要输出 Markdown，不要添加解释。JSON 格式：

{
  "frames": [
    {
      "title": "分镜板标题",
      "scene_summary": "该视频单元的动作和情绪摘要",
      "storyboard_panels": [
        {
          "panel_index": 1,
          "moment": "起始状态 / 核心动作 / 反应 / 结尾状态",
          "visual_goal": "这一格要让观众看清的可见状态变化",
          "composition": "景别、机位、主体位置、道具和空间锚点",
          "continuity_note": "与前后格保持连续的人物、场景、光源和动作承接"
        }
      ],
      "frame_prompt": "多格分镜板成图提示词，必须逐格写清第 1 格、第 2 格等内容，并按目标提示词语言和指定小节列清参考图约束、画面主体、空间道具、构图光线风格、一致性与避免项",
      "blocking_state": {
        "characters": [
          {
            "name": "人物名",
            "position": "分镜板整体的人物站位和空间关系",
            "facing": "主要朝向",
            "pose": "关键姿态变化"
          }
        ],
        "spatial_anchors": ["固定参考物"],
        "do_not_change": ["不能改变的空间关系"]
      },
      "lighting_state": {
        "time_of_day": "时间",
        "key_light": "主光源",
        "direction": "光源方向",
        "shadow": "阴影状态",
        "intent": "情绪意图"
      },
      "negative_prompt": "低质量、人体结构扭曲、多余肢体、面部不一致、空间关系错误、光影不连续、字幕、水印、乱码文字、文字编号"
    }
  ]
}

要求：
- `frames` 数量必须为 1；这一条 frame 表示一张完整分镜板图片，不是多个后端帧记录。
- 不要输出 `frame_number`、`segment_numbers`、`duration_secs`、`aspect_ratio` 或 `reference_image_ids`；后端会根据当前视频单元和用户选择补齐这些字段。
- 必须输出 `storyboard_panels`，数量必须在 2 到 6 之间；`panel_index` 必须从 1 连续递增。
- 分镜板格数由内容决定，不按固定模板或固定秒数硬切。每一格必须对应一个观众需要看清的可见状态变化；没有新状态变化就不要增加格子。
- 简单动作或静态情绪变化通常 2 格；一个清晰动作过程通常 3 格；动作加对白、听者反应或关系转折通常 3 到 4 格；复杂调度、移动、发现、反应和转场最多 5 到 6 格；action_bridge 单元通常 2 到 3 格，且只表现 `preserved_action`。
- `frame_prompt` 必须明确写出总格数，并逐格描述“第 1 格 / 第 2 格 / 第 3 格”或目标语言等价表达；每一格要写清主体动作、表情/视线、站位、道具、景别、机位、光源和与前后格的连续关系。
- `frame_prompt` 必须描述一张完整多格分镜板：画格按从左到右、从上到下阅读；根据画幅选择横排、竖排、2x2、2x3 或 3x2 等清晰版式；画格大小必须足够读清人物表情和动作，不要拥挤。
- 分镜板允许清晰画格边界和版式分隔线，但不要生成文字编号、字幕、水印、乱码标注、素材展示墙、无关拼贴或截图列表。
- 如果 `current_storyboard_frame.video_unit.unit_type` 是 `content`，分镜板应覆盖本内容片段的起始状态、核心动作、人物关系压力、听者/被影响者反应和结尾稳定状态。
- 如果 `current_storyboard_frame.video_unit.unit_type` 是 `transition`，分镜板只表达前一片段尾态到后一片段首态之间的桥接动作、空间移动、视线转移、情绪余波或转场姿态。不得新增关键剧情，不得重复前后内容片段的主动作。
- 必须读取 `current_storyboard_frame.video_unit`、相关源片段的 `video_intent`、`emotional_arc`、`performance_plan` 和 `material_coverage`，把走位、身体重心、视线、手部、停顿反应、镜头关系和素材缺口拆成可读画格。
- 如果原文有对白或关系动作，至少一个画格要让说话人、听者或被影响者的相对位置和可见反应进入画面；不要只画说话人口型。
- `frame_prompt` 的参考图说明只能使用后端提供的 `constraints.reference_slots` 顺序：第一项是“参考图 1”，第二项是“参考图 2”。不要使用候选素材序号、展示标签、字段名或内部编号作为参考图编号。
- `frame_prompt` 和 `negative_prompt` 的自然语言正文禁止出现 `image_id`、`image id`、`图片 ID`、`参考图 ID`、`available_assets`、候选素材标签、`manual_library_asset_*` 或任何原始图片数字 ID；只能用“参考图 1/参考图 2”描述已绑定参考图的用途。
- 不要输出 `reference_image_ids`；后端会按用户选择和当前工作流补齐真实图片 ID。
- 选择参考图时优先使用能稳定人物身份、场景、道具、服装或风格的素材。
- 如果用户指定风格中包含“风格模板”或“风格关键词”，构图光线风格小节必须保留其中的中文风格名和关键英文风格名，并吸收其光影、色彩、构图、镜头、材质、氛围和细节要求。
- `blocking_state` 必须写清分镜板整体的人物站位、朝向、姿态变化、左右/前后关系、主要道具和固定参照物。
- `lighting_state` 必须写清分镜板整体主光方向、光源类型、阴影状态和画面情绪；各画格光源方向必须连续。
- 如果原文包含极端暴力、血腥动态细节或自伤危险动作，保留剧情功能但改成可生成的静态替代表达：人物反应、危险物保持安全距离、非血腥痕迹、冷色压迫氛围或环境线索；不要描写伤害动作正在发生或即将发生。
- `negative_prompt` 只写本分镜板特有避免项；必须包含避免字幕、水印、乱码文字和文字编号，不要重复过长通用负面词。
