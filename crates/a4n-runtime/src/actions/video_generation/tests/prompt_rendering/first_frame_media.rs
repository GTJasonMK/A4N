#[test]
fn first_frame_chain_provider_prompt_keeps_material_refs_out_of_image() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "shot": {
            "title": "回望暗廊",
            "scene_summary": "小宦官从上一段尾帧继续回望暗处",
            "blocking_plan": {"start": "上一段尾帧的暗廊入口前", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
            "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承上一段尾帧"},
            "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
            "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
            "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "形成可采集尾帧"}],
            "reference_binding": {"bindings": [{"image_id": 11, "entity": "小宦官", "role": "character", "usage": "保持脸和服装一致", "must_keep": "同一人物"}], "coverage_check": "参考图 11 已绑定"},
            "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "已覆盖", "revision": "无需修订"}
        },
        "start_first_frame": {
            "title": "上一段尾帧",
            "scene_summary": "小宦官停在暗廊入口前",
            "frame_prompt": "小宦官靠近暗廊入口但没有越过，回望右后方黑暗走廊。",
            "blocking_state": {"characters": [{"name": "小宦官", "position": "暗廊入口前"}]},
            "lighting_state": {"key_light": "烛光", "direction": "画面右侧"}
        }
    }));

    let prompt = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect("first-frame-chain provider prompt should compile");

    assert!(prompt.contains("本次起步输入图片是视频第 0 秒画面"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
    assert!(prompt.contains("本请求按顺序输入 1 张参考图"));
    assert!(!prompt.contains("素材参考图只用于"));
    assert!(!prompt.contains("【首帧与结尾状态】"));
    assert!(!prompt.contains("【片段独立生成上下文】"));
    assert!(!prompt.contains("0-4秒：中景轻微漂移，回头并靠近暗廊入口，形成可采集尾帧。"));
    assert!(!prompt.contains("【首帧串联规则】"));
    assert!(!prompt.contains("首帧串联模式"));
    assert!(!prompt.contains("上一段视频自动采集"));
    assert!(!prompt.contains("上一段尾帧"));
    assert!(!prompt.contains("系统采集"));
    assert!(!prompt.contains("供系统"));
    assert!(!prompt.contains("generated_image #"));
    assert!(!prompt.contains("专用 `image` / `first_frame_image_url`"));
    assert!(!prompt.contains("参考图 #11 不能作为视频首帧"));
    assert!(!prompt.contains("不是首帧输入"));
    assert!(!prompt.contains("不要把参考图 #11 的原始画面当作视频开头画面"));
    assert!(!prompt.contains("【首尾帧状态】"));
}

#[test]
fn provider_prompt_uses_actual_bound_media_for_input_image_section() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    let engine = prompt_engine();
    let prompt = super::build_video_generation_provider_prompt_with_binding(
        &engine,
        &row,
        ProviderPromptImageBinding {
            has_first_frame_image: true,
            has_last_frame_image: false,
            reference_image_count: 0,
            has_storyboard_reference_image: false,
        },
    )
    .expect("provider prompt should compile");

    assert!(prompt.contains("- 首帧输入图片是视频第 0 秒画面"));
    assert!(!prompt.contains("本请求按顺序输入"));
    assert!(!prompt.contains("尾帧输入图片是视频结尾目标"));
}

#[test]
fn first_frame_chain_provider_prompt_allows_tail_frame_start_without_frame_metadata() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "shot": {
            "title": "回望暗廊",
            "scene_summary": "小宦官从上一段尾帧继续回望暗处",
            "blocking_plan": {"start": "上一段尾帧的暗廊入口前", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
            "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承上一段尾帧"},
            "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
            "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
            "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "形成可采集尾帧"}],
            "reference_binding": {"bindings": [], "coverage_check": "无参考图"},
            "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "无素材引用", "revision": "无需修订"}
        },
        "start_first_frame": null
    }));

    let prompt = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect("tail-frame chained provider prompt should compile without start_first_frame");

    assert!(prompt.contains("本次起步输入图片是视频第 0 秒画面"));
    assert!(prompt.contains("必须从这张图自然推进"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(!prompt.contains("上一段视频自动采集"));
    assert!(!prompt.contains("上一段尾帧"));
    assert!(!prompt.contains("起始帧由系统"));
    assert!(!prompt.contains("首帧串联规则"));
    assert!(!prompt.contains("供系统"));
    assert!(!prompt.contains("不得改用素材参考图或重新设计首帧"));
}
