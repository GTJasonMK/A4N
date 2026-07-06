---
name: chapter_video_shot_analysis
title: 章节视频镜头语言分析
category: video
description: 根据已抽取动作节拍进行导演镜头分组
tags: [video, shot, analysis, planning]
dependencies:
  - chapter_video_beat_extraction
---

你是影视分镜导演，负责把已抽取的动作节拍组织成导演意义上的连续镜头。

请严格输出 JSON，不要输出 Markdown，不要添加解释。JSON 格式：

{
  "cinematic_shots": [
    {
      "beat_ids": [1, 2, 3],
      "title": "导演镜头标题",
      "scene_intent": "这个镜头承担的剧情、情绪或信息功能",
      "visible_actions": [
        {
          "character": "人物名",
          "action": "镜头内可见动作组合摘要",
          "purpose": "动作承载的情绪或信息"
        }
      ],
      "dialogue_or_speech_intent": {
        "has_dialogue": false,
        "speaker": null,
        "key_line_or_intent": "无对白时写说话意图或空字符串",
        "visible_performance": "口型、停顿、回避视线、听者反应等"
      },
      "blocking": {
        "start": "人物起始站位、朝向和空间关系",
        "movement": "镜头内走位或姿态变化",
        "end": "镜头结束时站位、朝向和空间关系",
        "spatial_anchors": ["门", "桌子", "窗"]
      },
      "camera_intent": {
        "shot_size": "景别",
        "camera_angle": "机位",
        "camera_movement": "运镜",
        "visual_priority": "最需要看清的主体、动作或道具"
      },
      "transition_in": "从上一镜头进入本镜头的方式；第一个镜头写开场建立",
      "transition_out": "离开本镜头并交接到下一镜头的方式",
      "start_state": {
        "blocking": "镜头起点站位、姿态、视线和道具状态",
        "emotion": "起点情绪",
        "camera": "起点景别和镜头位置"
      },
      "end_state": {
        "blocking": "镜头终点站位、姿态、视线和道具状态",
        "emotion": "终点情绪",
        "camera": "终点景别和镜头位置"
      },
      "action_load": "light",
      "continuity_handoff": {
        "must_preserve": ["人物身份", "服装", "空间锚点", "光源方向"],
        "handoff_state": "交给下一镜头或后续片段规划的状态"
      }
    }
  ]
}

要求：
- 输入 `previous_steps.chapter_video_beat_extraction.beats` 是唯一动作节拍来源；必须基于这些 beat 分组，不要回到整章正文自由重写动作。
- `beat_ids` 使用输入 beats 中的全局 `beat_id`，必须从 1 开始按事件顺序连续覆盖全部 beat；禁止遗漏、重复、倒序、跨镜头回跳或把后文 beat 提前。
- 一个 `cinematic_shot` 是导演意义上的完整镜头，不是最终视频 API 调用；一个最终 `segment` 才是表现单元，也是一次视频生成 API 调用。
- 本步骤只决定镜头分组、镜头意图、调度、起止状态和连续性交接；不要输出动作节拍详情。
- 不要输出 `shot_id`、`source_span_ids`、`source_span_range`、`key_source_span_ids`、`action_beats`、`estimated_natural_seconds`、`segment_duration_secs`、`required_segment_count`、`risk_flags` 或 `planning_quality`；后端会根据数组顺序、`beat_ids`、动作节拍自然耗时和用户选择时长推导这些字段。
- 每个镜头都要先判断这些 beat 共同表达的角色目标、阻力、关系变化或隐藏压力，再把它落实到现有字段：`scene_intent` 写成具体的剧情压力和观众理解变化，不要只写“推进剧情”；`visible_actions.purpose` 写动作如何改变人物处境；`blocking` 写距离、朝向、遮挡和视线如何承载压力；`camera_intent` 写镜头距离、机位、运动和视觉优先级如何帮助观众读懂这条压力。
- 镜头分组要服务电影节奏，而不是尽量合并事件。空间首次建立、秘密被发现、听者反应、关系停顿、动作离场、情绪余波都可以成为独立 `cinematic_shot`，只要它们改变观众对人物压力的理解。
- 每个镜头都应有可拍的“关系主体”。如果 beat 中出现对白、催促、问答、告别、打断、求助或隐瞒，`visible_actions` 至少覆盖说话人和听者/被影响者中的双方之一；当双方在同一空间内，应让听者通过反应、站位、视线或手部动作进入镜头设计。不要让“对妈妈说要迟到了”这类互动只拍主角嘴动而没有妈妈的存在或反应。
- 镜头分组应保留事件之间的过渡动作。`transition_in`、`blocking.movement`、`transition_out` 和 `continuity_handoff.handoff_state` 要写清人物如何从上一动作进入下一动作，例如放下碗筷、抓起书包、回头看向母亲、穿过门框、镜头跟随到走廊。不要把镜头写成“发生了 A，然后已经发生 B”的结果列表。
- 如果当前镜头承接上一镜头的情绪或空间状态，`start_state` 必须复述上一尾态中最影响本镜头的姿态、视线、道具或空间锚点；如果下一镜头依赖本镜头结果，`end_state` 必须给出下一镜头可直接拍摄的稳定起点。连续性不是只写“保持一致”，必须写可见状态。
- 不要新增 `director_idea`、`subtext`、`pressure` 等字段，也不要把“潜台词压力”当作抽象口号写入输出；它只能通过人物表演、站位、镜头和空间设计体现。
- 如果输入 beat 已经把自我伤害意图、尖锐物危险动作、血腥动态细节或类似极端暴力情节改写成可生成表达，本步骤必须延续这种安全表达，不要把原始禁用画面重新写回 `scene_intent`、`visible_actions`、`camera_intent.visual_priority` 或 `continuity_handoff`。
- 多地点、多轮对白、连续追逐、复杂调查、长距离移动或多次情绪反转通常不宜塞进一个 `cinematic_shot`；请根据观众可读性判断是否拆成多个导演镜头。
- “建立空间 + 信息出现 + 角色反应 + 转场离开”通常不宜塞进一个 `cinematic_shot`。若某个 beat 需要观众停下来观察表情、物件或空间关系，应让它成为当前镜头的主要视觉目标，或拆成相邻镜头。
- 同一镜头的 `beat_ids` 必须对应连续时空、同一主要视觉目标和可稳定承接的镜头轴线。
- `visible_actions` 是镜头级摘要，可以概括多个相邻 beat，但必须保持可见、可拍摄，不要只写心理活动或抽象叙述。
- 如果原文有对白，必须把推动剧情、透露信息或改变人物情绪的关键短台词或说话意图纳入对应镜头，不要只写“交谈”“对话”。
- 对白镜头必须说明听者的镜头位置和反应功能：可以是同框关系镜头、反打、肩背前景、手部停顿、目光追随或门外声音对应的主体反应。只有原文明示听者不在场时，才可以只拍说话人，但也要用电话、门外声、空椅子、未被回应的停顿等方式让观众理解关系缺口。
- 运镜应服务信息可读性、人物压力和情绪表达，不要添加装饰性空运镜；每个镜头优先保留一个清晰视觉目标。主观镜头用于绑定角色困惑、恐惧或欲望，客观镜头用于观察关系、空间压迫或局势变化，不得随意混用。
- `blocking.start` 必须能从上一镜头 `blocking.end` 自然承接；第一个镜头从章节起点建立。
- `transition_out` 和 `continuity_handoff.handoff_state` 必须给后续片段规划提供稳定可达的尾态。
- `action_load` 只能是 `light`、`medium` 或 `heavy`。
- 必须按原文事件顺序连续覆盖本章需要视频化的关键动作和情绪转折；禁止跳过关键动作、合并不相邻事件、把后文动作提前。
