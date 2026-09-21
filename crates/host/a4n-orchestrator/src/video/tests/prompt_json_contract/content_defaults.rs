#[test]
fn parses_video_frame_prompt_with_self_harm_intent_in_direct_text() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图 1 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物与空间关系，并为后续自残动作铺垫。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect("self-harm wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_frame_prompt_with_gore_detail_in_direct_text() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图 1 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物，血从指缝渗出并有血迹扩散的细节要改成可生成的环境压迫感。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物与空间关系。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect("gore wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_frame_prompt_with_explicit_self_harm_execution_in_direct_text() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图 1 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手拿着铁片刺入眼睛，表情痛苦。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物与空间关系。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect("self-harm execution wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_frame_prompt_with_avoided_self_harm_term() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图 1 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续，避免出现血迹、绷带、眼部伤口或任何自残迹象。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误、自残画面"
          }]
        }"#,
    )
    .expect("avoidance wording should not be treated as self-harm execution");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].frame_number, 1);
}

#[test]
fn parses_video_frame_prompts_with_missing_frame_numbers_by_order() {
    let rows = parse_video_frame_prompts(
        r#"{
          "frames": [
            {
              "title": "首帧",
              "scene_summary": "角色抵达门前",
              "frame_prompt": "夜晚室内外交界处的紧闭木门占据画面中央，角色站在门前偏左，半侧身面向木门，右手自然垂在身侧，门框、铜环和地面阴影作为固定参照物，月光从画面左侧落下，整体保持悬疑写实风格和稳定构图。",
              "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "站立"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
              "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
              "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
            },
            {
              "title": "尾帧",
              "scene_summary": "角色进入室内",
              "frame_prompt": "角色已经进入昏暗房间内，身体停在门内右侧半步位置，肩膀微微收紧并回望门口，打开的木门、门框和地面月光斑作为固定参照物，月光仍从画面左侧照入，室内阴影柔和，结尾画面稳定并可承接下一段。",
              "blocking_state": {"characters": [{"name": "角色", "position": "室内门边", "facing": "看向室内", "pose": "停步"}], "spatial_anchors": ["门"], "do_not_change": ["人物仍靠近门"]},
              "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "昏暗阴影", "intent": "悬疑"},
              "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
            }
          ]
        }"#,
    )
    .expect("frame_number should be derived from frame order");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].frame_number, 1);
    assert_eq!(rows[1].frame_number, 2);
}

#[test]
fn rejects_model_owned_reference_image_ids_in_frame_prompt_output() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![59],
    }];
    let err = parse_video_frame_prompts_with_defaults(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "夜晚室内外交界处的紧闭木门占据画面中央，角色站在门前偏左，半侧身面向木门并伸出右手准备推门，袖口和手部清晰可见，门框、铜环和地面阴影作为固定参照物，月光从画面左侧落下，整体保持悬疑写实风格和稳定构图。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误",
            "reference_image_ids": [1]
          }]
        }"#,
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("model-owned frame reference ids should fail visibly");

    let message = err.to_string();
    assert!(message.contains("unknown field `reference_image_ids`"));
    assert!(message.contains("输出预览"));
}

#[test]
fn rejects_duplicate_backend_reference_image_ids() {
    let defaults = [VideoFramePromptDefaults {
        frame_number: 1,
        duration_secs: 6,
        aspect_ratio: "16:9",
        segment_numbers: vec![1],
        reference_image_ids: vec![3, 3],
    }];
    let err = parse_video_frame_prompts_with_defaults(
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
        &defaults,
        PromptOutputLanguage::ZhCn,
    )
    .expect_err("backend-derived duplicate reference image ids should be rejected");

    assert!(err.to_string().contains("重复 ID：3"));
}
