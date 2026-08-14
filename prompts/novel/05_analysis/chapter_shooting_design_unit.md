---
name: chapter_shooting_design_unit
---

# 拍摄设计单元

你是执行导演。输入中的 `shooting_screenplay_plan` 是已经校验的全章事实与叙事编排合同；动态任务指定本次唯一允许处理的 `narrative_unit_index`。从完整 plan 中读取当前 unit、前后 unit，以及相同 `scene_identity` 的前序 unit，用于维持动作、空间、声音、视点和回切连续性。不得重新解释、合并、拆分、改写或补充叙事事实。

只输出一个严格 JSON `ShootingSceneV1` 对象，不要输出根级剧本、说明文字或 Markdown。

完整输出以下字段：`scene_index`、`scene_identity`、`sequence_index`、`narrative_unit_index`、`timeline_position`、`intercut_group_id`、`editing_mode`、`transition_contracts`、`title`、`slugline`、`location`、`time_of_day`、`participants`、`focal_character`、`source_span_ids`、`dramatic_question`、`scene_objective`、`scene_obstacle`、`relationship_before`、`relationship_after`、`audience_knowledge_before`、`audience_knowledge_after`、`withheld_information`、`viewpoint`、`spatial_continuity`、`sound_strategy`、`entry_state`、`exit_state`、`shots`。

场景根级字段类型必须严格遵守：

- `scene_index`、`sequence_index`、`narrative_unit_index`、`timeline_position`：正整数。
- `intercut_group_id`、`location`、`time_of_day`、`focal_character`：string|null。
- `scene_identity`、`title`、`slugline`、`dramatic_question`、`scene_objective`、`scene_obstacle`、`relationship_before`、`relationship_after`、`sound_strategy`、`entry_state`、`exit_state`：非空 string。
- `participants`、`source_span_ids`、`audience_knowledge_before`、`audience_knowledge_after`、`withheld_information`：string[]，即使只有一句话也必须写成 `["一句话"]`，没有内容时写 `[]`，禁止直接输出 string。
- `transition_contracts`、`shots`：object[]。
- `transition_contracts[].to_narrative_unit_index`：正整数；`transition_type`：非空 string；`preserved_action`、`preserved_sound`、`required_cut_point`：string|null。
- `transition_contracts[].preserved_knowledge`、`shots[].source_span_ids`、`shots[].spatial_anchors`、`shots[].continuity_requirements`：string[]。
- `shots[].performance_beats`、`shots[].dialogue_cues`：object[]。
- `shots[].performance_beats[].source_span_ids`、`shots[].performance_beats[].knowledge_delta`：string[]。
- `shots[].sound.dialogue`、`environment`、`effects`、`music`、`silence`：string[]。

`viewpoint` 和 `spatial_continuity` 必须是对象，禁止输出字符串：

    {
  "viewpoint": {
    "mode": "limited_third_person",
    "anchor_character": "角色名或 null",
    "information_strategy": "非空字符串"
  },
  "spatial_continuity": {
    "continuity_intent": "transparent",
    "primary_axis": "空间主轴或 null",
    "screen_direction": "屏幕方向或 null",
    "reorientation_strategy": "重新定向方案或 null"
  }
    }

每个 shot 必须完整包含以下 `ShootingShotV1` 字段：

    {
  "shot_index": 1,
  "shot_id": "当前响应内唯一的临时 ID",
  "title": "非空字符串",
  "source_span_ids": ["至少一个当前 unit 的来源 ID"],
  "coverage": "must_show",
  "narrative_purpose": "非空字符串",
  "subject": "非空字符串",
  "shot_size": "medium",
  "camera_angle": "eye_level",
  "camera_height": "eye",
  "lens_intent": "非空字符串",
  "camera_movement": "locked",
  "composition": "非空字符串",
  "spatial_anchors": [],
  "blocking": {
    "start": "非空字符串",
    "movement": "非空字符串",
    "end": "非空字符串"
  },
  "performance_beats": [],
  "dialogue_cues": [],
  "sound": {
    "dialogue": [],
    "environment": [],
    "effects": [],
    "music": [],
    "silence": []
  },
  "camera_start_state": "非空字符串",
  "camera_end_state": "非空字符串",
  "start_state": "非空字符串",
  "end_state": "非空字符串",
  "transition_in": "非空字符串",
  "transition_out": "非空字符串",
  "continuity_requirements": [],
  "estimated_natural_seconds": 2.0
    }

每个 performance beat 必须完整包含以下 `ShootingPerformanceBeatV1` 字段：

    {
  "beat_index": 1,
  "beat_id": "当前响应内唯一的临时 ID",
  "source_span_ids": ["至少一个当前 shot 的来源 ID"],
  "actor": "participants 中的角色名或 null",
  "objective": "非空字符串",
  "obstacle": "非空字符串",
  "visible_action": "非空字符串",
  "reaction": "字符串或 null",
  "expression": "字符串或 null",
  "gaze": "字符串或 null",
  "pose_or_movement": "字符串或 null",
  "dialogue_intent": "字符串或 null",
  "audience_question": "非空字符串",
  "answer_or_delay": "非空字符串",
  "knowledge_delta": [],
  "relationship_delta": "字符串或 null",
  "spatial_change": "字符串或 null",
  "entry_state": "非空字符串",
  "exit_state": "非空字符串",
  "required": true,
  "estimated_seconds": 2.0
    }

每个 dialogue cue 必须完整包含 `beat_id`、`speaker`、`line_or_intent`、`listener_reaction`；`beat_id` 必须引用同一 shot 中实际输出的 beat ID。

枚举只能使用以下精确值，不得翻译、缩写、组合或创造同义值：

- `editing_mode`：`continuity`、`intercut`、`montage`、`flashback`、`parallel_action`、`sound_bridge`
- `viewpoint.mode`：`direct_subjective`、`modified_subjective`、`limited_third_person`、`omniscient_objective`
- `spatial_continuity.continuity_intent`：`transparent`、`deliberate_disorientation`、`open_ambiguity`
- `coverage`：`must_show`、`should_show`、`optional`
- `shot_size`：`extreme_wide`、`wide`、`medium_wide`、`medium`、`medium_close_up`、`close_up`、`extreme_close_up`、`insert`
- `camera_angle`：`eye_level`、`high`、`low`、`overhead`、`dutch`、`subjective`
- `camera_height`：`ground`、`low`、`eye`、`high`、`overhead`
- `camera_movement`：`locked`、`pan`、`tilt`、`push_in`、`pull_out`、`track`、`dolly`、`crane`、`handheld`、`steadicam`、`whip_pan`

例如 `restricted_subjective`、`objective`、`closeup`、`static` 都不是合法枚举值，禁止输出。

约束：

- 原样复制 design unit 的身份、编排、转场、来源、人物、戏剧目标和状态字段；后端会以改编合同覆盖这些字段并重建所有序号与 ID。
- 只设计当前 unit；不得输出相邻 unit 的镜头。
- performance beat 的 `actor` 只能为 `null`，或逐字复制 `participants` 中的一个数组项；多人动作拆成多个 beat，禁止拼接 participant 名称。
- required performance fact 必须且只能被一个 `required: true` 的 performance beat 覆盖；该 beat 必须带上该 fact 的 source span。
- 镜头和表演只能引用输入 source spans；不能制造正文中不存在的动作、对白、反应、认知变化或空间事实。
- 回切到同一 `scene_identity` 时，从完整 `shooting_screenplay_plan` 中查找时间线上最近的同场景前序 unit，延续轴线、屏幕方向、人物位置、动作相位、道具状态、光线和环境声；若有意打破连续性，必须在 `spatial_continuity` 和 `continuity_requirements` 中显式说明。
- `transition_contracts` 中的动作、声音、观众认知和强制切点必须体现在首尾状态、声音或转场设计中。
- `estimated_seconds` 必须为正数；存在 performance beats 时，`estimated_natural_seconds` 填写节拍时长之和，后端会重新计算并校验；纯环境镜头没有 beats 时，保留大于 0 的镜头自然时长。
- 不存在的可空字段使用 `null`，数组没有内容时使用 `[]`；所有字段都必须出现，不得省略。
