#[test]
fn parses_video_frame_prompts() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "夜晚室内外交界处的紧闭木门占据画面中央，角色站在门前偏左，半侧身面向木门并伸出右手准备推门，袖口和手部清晰可见，门框、铜环和地面阴影作为固定参照物，月光从画面左侧落下，整体保持悬疑写实风格和稳定构图。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect("frame prompts should parse");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].frame_number, 1);
    assert_eq!(rows[0].duration_secs, 6);
    assert_eq!(rows[0].segment_numbers, vec![1]);
}

#[test]
fn parses_video_frame_prompt_with_reference_slot_label() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图 1 只作为门厅场景、木门结构和月光方向的一致性约束，不把素材图当作最终画面。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect("spaced reference labels should be valid direct prompt text");

    assert_eq!(rows[0].reference_image_ids, vec![3]);
}

#[test]
fn rejects_video_frame_prompt_with_model_owned_frame_number() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "frame_number": 1,
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "夜晚室内外交界处的紧闭木门占据画面中央，角色站在门前偏左，半侧身面向木门并伸出右手准备推门，袖口和手部清晰可见，门框、铜环和地面阴影作为固定参照物，月光从画面左侧落下，整体保持悬疑写实风格和稳定构图。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("frame_number must be backend-derived");

    assert!(err.to_string().contains("unknown field `frame_number`"));
}

#[test]
fn parses_video_frame_prompt_without_model_owned_reference_image_ids() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "夜晚室内外交界处的紧闭木门占据画面中央，角色站在门前偏左，半侧身面向木门并伸出右手准备推门，袖口和手部清晰可见，门框、铜环和地面阴影作为固定参照物，月光从画面左侧落下，整体保持悬疑写实风格和稳定构图。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect("reference_image_ids is backend-derived for frame prompts");

    assert_eq!(rows[0].reference_image_ids, vec![3]);
}
