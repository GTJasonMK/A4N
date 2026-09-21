#[test]
fn first_frame_video_prompt_user_uses_first_frame_specific_instruction() {
    let engine = prompt_renderer();
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}]}),
        &json!({"frames": [{"frame_number": 1}]}),
        &json!({"segment_number": 1}),
        None,
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([]),
    ))
    .expect("first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert_eq!(
        payload["task"]["name"],
        json!("chapter_video_first_frame_video_prompt"),
        "first-frame chain should not reuse generic video task"
    );
    assert_eq!(
        payload["output_contract"]["prompt_name"],
        json!("chapter_video_first_frame_video_prompt")
    );
}

#[test]
fn first_frame_video_prompt_payload_uses_positive_input_frame_usage_for_start_only() {
    let engine = prompt_renderer();
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}]}),
        &json!({
            "frames": [{
                "frame_number": 1,
                "scene_summary": "角色坐在窗边，看向窗外。",
                "frame_prompt": "窗边首帧画面"
            }]
        }),
        &json!({
            "segment_number": 1,
            "duration_secs": 5,
            "source_span_ids": ["ch1:s1:f1"],
            "emotional_arc": {"end": "角色看向窗外陷入沉思"},
            "continuity_handoff": "角色保持坐姿，视线停在窗外"
        }),
        None,
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::En,
        5,
        "16:9",
        &json!([]),
    ))
    .expect("first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert!(
        payload["input_frames"].get("start_frame").is_some(),
        "start frame should be present"
    );
    assert!(
        payload["input_frames"].get("end_frame").is_none(),
        "first-frame chain start-only request must not expose an end_frame placeholder"
    );
    assert_eq!(
        payload["constraints"]["input_frame_usage"]["opening_source"],
        json!("input_frames.start_frame")
    );
    assert_eq!(
        payload["constraints"]["input_frame_usage"]["ending_source"],
        json!("current_segment_end_state")
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["ending_instruction"]
            .as_str()
            .is_some_and(|text| text.contains("current_segment.emotional_arc.end")),
        "ending instruction should point to segment-derived end state"
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["upload_status_rule"]
            .as_str()
            .is_some_and(
                |text| text.contains("资源是否存在") || text.contains("resource availability")
            ),
        "payload should expose the generic upload-status rule"
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["ending_instruction"]
            .as_str()
            .is_some_and(|text| !text.contains("没有输入尾帧") && !text.contains("无输入尾帧")),
        "ending instruction must describe segment-derived end state, not missing tail-frame status"
    );
    assert!(!user.contains("No input end frame"));
}

#[test]
fn first_frame_video_prompt_payload_uses_reference_slots_without_internal_asset_ids() {
    let engine = prompt_renderer();
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}]}),
        &json!({
            "frames": [{
                "frame_number": 1,
                "scene_summary": "角色坐在窗边，看向窗外。",
                "frame_prompt": "窗边首帧画面"
            }]
        }),
        &json!({
            "segment_number": 1,
            "duration_secs": 5,
            "source_span_ids": ["ch1:s1:f1"],
            "emotional_arc": {"end": "角色看向窗外陷入沉思"},
            "continuity_handoff": "角色保持坐姿，视线停在窗外"
        }),
        None,
        &json!([
            {
                "image_id": 59,
                "display_order": 1,
                "display_label": "参考图 1",
                "asset_id": "manual_library_asset_1",
                "source_id": "manual_library_asset_1",
                "candidate_label": "manual_library_asset_1",
                "source_display_label": "参考图 1",
                "project_id": 9,
                "profile_id": 7,
                "role": "chapter_material",
                "label": "manual_library_asset_1",
                "description": "林清辞角色参考 manual_library_asset_1",
                "prompt_excerpt": "清晨教室窗边的角色主图",
                "selection_reason": "manual_library",
                "profile_kind": "asset_id 59",
                "is_primary": true,
                "is_favorite": false
            }
        ]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([59]),
    ))
    .expect("first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert!(payload.get("available_assets").is_none());
    assert!(payload["constraints"].get("reference_image_ids").is_none());
    assert_eq!(
        payload["constraints"]["reference_slots"][0]["slot_label"],
        json!("参考图 1")
    );
    let asset = payload["reference_asset_summaries"]
        .as_array()
        .and_then(|items| items.first())
        .expect("reference asset summary should exist");
    let serialized = serde_json::to_string(asset).expect("asset summary should serialize");

    assert!(asset.get("image_id").is_none());
    assert!(asset.get("display_label").is_none());
    assert!(asset.get("display_order").is_none());
    assert!(asset.get("project_id").is_none());
    assert!(asset.get("profile_id").is_none());
    assert!(asset.get("asset_id").is_none());
    assert!(asset.get("source_id").is_none());
    assert!(asset.get("candidate_label").is_none());
    assert!(asset.get("source_display_label").is_none());
    assert!(!serialized.contains("manual_library_asset_1"));
    assert!(!serialized.contains("asset_id"));
}

#[test]
fn first_frame_video_prompt_payload_uses_previous_shot_as_later_segment_opening() {
    let engine = prompt_renderer();
    let previous_shot = json!({
        "title": "上一段",
        "scene_summary": "角色停在门口，手握钥匙回头看向走廊。",
        "visual_prompt": "[主体定义与素材绑定]\n角色停在门口。\n\n[片段内容以及前因后果]\n上一段结尾停在门口。\n\n[镜头分镜]\n镜头 1：平视中景保持角色停在门口，手握钥匙并回头看向走廊。\n镜头 2：镜头轻微稳定收束，角色维持门口站位，走廊保持在画面右侧。\n\n[画面风格描述]\n电影感写实。\n\n[连续性与结尾状态]\n角色手握钥匙，走廊在右侧。\n\n[限制与避免项]\n避免字幕。"
    });
    let user = build_first_frame_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": [{"segment_number": 1}, {"segment_number": 2}]}),
        &json!({
            "frames": [{
                "frame_number": 1,
                "scene_summary": "第 1 段起始画面",
                "frame_prompt": "第 1 段首帧"
            }]
        }),
        &json!({
            "segment_number": 2,
            "duration_secs": 5,
            "source_span_ids": ["ch1:s1:f2"],
            "emotional_arc": {"end": "角色推门进入"},
            "continuity_handoff": "角色进入门内，钥匙仍在手里"
        }),
        Some(&previous_shot),
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([]),
    ))
    .expect("later first-frame video user prompt should render");
    let payload = video_user_payload(&user);

    assert!(
        payload["input_frames"].get("start_frame").is_none(),
        "later first-frame chain prompt should not invent a missing start frame"
    );
    assert_eq!(
        payload["constraints"]["input_frame_usage"]["opening_source"],
        json!("previous_completed_shot")
    );
    assert!(
        payload["constraints"]["input_frame_usage"]["opening_instruction"]
            .as_str()
            .is_some_and(|text| text.contains("不得写资源缺失")),
        "opening instruction should forbid upload-status wording without exposing banned phrases"
    );
    assert!(
        payload["constraints"]["input_frame_usage"]
            .get("forbidden_upload_status_terms")
            .is_none(),
        "payload must not expose exact forbidden terms to the model"
    );
    assert!(!user.contains("previous_completed_shot_or_current_segment"));
}
