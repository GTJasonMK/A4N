#[test]
fn rejects_video_frame_prompt_with_internal_image_id_in_direct_text() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考门厅场景（image_id=3）约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("frame prompt text must not expose internal image ids");

    assert!(err.to_string().contains("内部图片 ID"));
    assert!(err.to_string().contains("image_id"));
}

#[test]
fn rejects_video_frame_prompt_with_chinese_image_id_in_direct_text() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考门厅场景的图片 ID 59 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("direct prompt text must not expose Chinese image id labels");

    let message = err.to_string();
    assert!(message.contains("内部图片 ID"));
    assert!(message.contains("图片 ID"));
}

#[test]
fn rejects_video_frame_prompt_with_candidate_asset_label_in_direct_text() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】候选素材 A 用于约束门厅场景、木门结构和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("candidate asset labels must not leak into direct prompt text");

    let message = err.to_string();
    assert!(message.contains("内部图片 ID"));
    assert!(message.contains("候选素材标签"));
}

#[test]
fn rejects_video_frame_prompt_with_manual_library_asset_token_in_direct_text() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】manual_library_asset_1 用于约束门厅场景、木门结构和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("manual library asset tokens must not leak into direct prompt text");

    let message = err.to_string();
    assert!(message.contains("内部图片 ID"));
    assert!(message.contains("manual_library_asset_*"));
}

#[test]
fn rejects_video_frame_prompt_with_compact_reference_number() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图12 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("compact reference labels must fail visibly");

    let message = err.to_string();
    assert!(message.contains("内部图片 ID"));
    assert!(message.contains("参考图后紧贴数字"));
}

#[test]
fn rejects_video_frame_prompt_with_reference_label_out_of_range() {
    let err = parse_video_frame_prompts(
        r#"{
          "frames": [{
            "title": "门前",
            "scene_summary": "角色推门进入",
            "frame_prompt": "【参考图约束】参考图 2 约束木门、门框和月光方向。【画面主体】角色站在门前偏左，半侧身面向木门，右手悬停在门把附近，袖口和手部清晰可见，表情谨慎。【空间与道具】门框、铜环和地面阴影作为固定参照物。【构图光线风格】月光从画面左侧落下，整体保持悬疑写实风格。【一致性与避免项】保持人物、木门、月光方向和空间关系连续。",
            "blocking_state": {"characters": [{"name": "角色", "position": "门前", "facing": "面向门", "pose": "伸手"}], "spatial_anchors": ["门"], "do_not_change": ["门在人物前方"]},
            "lighting_state": {"time_of_day": "夜晚", "key_light": "月光", "direction": "画面左侧", "shadow": "柔和阴影", "intent": "紧张"},
            "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余手指、面部不一致、空间关系错误"
          }]
        }"#,
    )
    .expect_err("reference labels must match bound reference count");

    assert!(err.to_string().contains("不存在的参考图 2"));
}
