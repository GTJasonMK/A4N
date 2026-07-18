#[test]
fn single_frame_prompt_user_exposes_reference_slots_not_internal_ids() {
    let engine = prompt_renderer();
    let user = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!([{
                "description": "林清辞角色参考"
            }]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([59, 80]),
        ),
        &json!({
            "frame_number": 2,
            "expected_segment_numbers": [1],
            "required_duration_secs": 6,
            "required_aspect_ratio": "16:9",
            "previous_frame_image": {
                "slot_index": 1,
                "slot_label": "参考图 1",
                "usage": "已生成的上一张边界帧画面"
            }
        }),
    )
    .expect("single frame prompt user should render");
    let payload = video_user_payload(&user);
    let constraints = payload["constraints"]
        .as_object()
        .expect("constraints should be an object");

    assert!(constraints.get("reference_image_ids").is_none());
    let slots = constraints
        .get("reference_slots")
        .and_then(Value::as_array)
        .expect("reference slots should be an array");
    assert_eq!(slots[0]["slot_label"], json!("参考图 1"));
    assert_eq!(slots[0]["role"], json!("previous_boundary_frame"));
    assert_eq!(slots[1]["slot_label"], json!("参考图 2"));
    assert!(
        serde_json::to_string(slots)
            .expect("reference slots should serialize")
            .find("59")
            .is_none()
    );
    let available_assets_text = serde_json::to_string(&payload["available_assets"])
        .expect("available assets should serialize");
    assert!(!available_assets_text.contains("候选素材"));
    assert!(!available_assets_text.contains("display_label"));
}

#[test]
fn single_frame_prompt_user_rejects_duplicate_backend_reference_ids() {
    let engine = prompt_renderer();
    let err = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([59, 59]),
        ),
        &json!({
            "frame_number": 1,
            "expected_segment_numbers": [1],
            "required_duration_secs": 6,
            "required_aspect_ratio": "16:9"
        }),
    )
    .expect_err("duplicate backend reference ids should fail before LLM request construction");

    assert!(format!("{err}").contains("重复图片 ID：59"));
}

#[test]
fn single_frame_prompt_user_rejects_missing_previous_frame_reference_slot() {
    let engine = prompt_renderer();
    let err = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": [{"segment_number": 1}]}),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([]),
        ),
        &json!({
            "frame_number": 2,
            "expected_segment_numbers": [1],
            "required_duration_secs": 6,
            "required_aspect_ratio": "16:9",
            "previous_frame_image": {
                "slot_index": 1,
                "slot_label": "参考图 1",
                "usage": "已生成的上一张边界帧画面"
            }
        }),
    )
    .expect_err("previous frame image requires a backend reference slot");

    assert!(format!("{err}").contains("上一帧参考图已存在"));
}

#[test]
fn single_frame_prompt_user_rejects_missing_system_fields() {
    let engine = prompt_renderer();
    let err = build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &json!({"segments": []}),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &json!([]),
        ),
        &json!({
            "frame_number": 1,
            "expected_segment_numbers": [1],
            "required_aspect_ratio": "16:9"
        }),
    )
    .expect_err("missing derived duration field must fail before LLM request construction");

    assert!(
        format!("{err}").contains("required_duration_secs"),
        "unexpected error: {err}"
    );
}
