#[test]
fn video_prompt_user_includes_action_load_and_temporal_capacity_rules() {
    let engine = prompt_renderer();
    let user = build_boundary_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": []}),
        &json!({"frames": []}),
        &json!({"segment_number": 1}),
        None,
        &json!([]),
        "风格模板：京阿尼动画风格",
        PromptOutputLanguage::ZhCn,
        10,
        "16:9",
        &json!([]),
    ))
    .expect("video prompt user prompt should render");

    assert!(user.contains("\"action_load_rule\""));
    assert!(user.contains("\"style_preservation_rule\""));
    assert!(user.contains("风格模板：京阿尼动画风格"));
    assert!(user.contains("不要配背景音乐"));
    assert!(user.contains("duration_secs 内自然完成且能被观众读懂的动作"));
    assert!(user.contains("中文输出密度：视频 visual_prompt 至少 80 个字符"));
    assert!(user.contains("最后一个镜头必须稳定到本段结尾状态"));
    let payload = video_user_payload(&user);
    assert_eq!(payload["constraints"]["selected_duration_secs"], json!(10));
    assert!(
        !contains_json_key(&payload, "duration_secs_options"),
        "final video prompt should not expose selectable duration options"
    );
    assert!(
        !contains_json_key(&payload, "default_duration_secs"),
        "final video prompt should use selected_duration_secs, not default duration"
    );
    assert_eq!(
        payload["output_contract"]["dialogue_cue_allowed_fields"],
        json!(VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS),
        "final video prompt should expose the dialogue cue field whitelist in the structured contract"
    );
    assert!(
        payload["output_contract"]["dialogue_cue_field_rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("dialogue_cue_allowed_fields")),
        "dialogue cue field rule should point the model at the structured whitelist"
    );
}

#[test]
fn final_video_prompt_payload_hides_internal_source_context_fields() {
    let engine = prompt_renderer();
    let user = build_boundary_video_prompt_user(final_video_context!(
            &engine,
            &json!({
                "segments": [{
                    "segment_number": 1,
                    "source_span_ids": ["ch1:s1:f1"],
                    "reference_image_ids": [59],
                    "scene_summary": "角色在教室窗边回头。"
                }]
            }),
            &json!({
                "frames": [{
                    "frame_number": 1,
                    "source_span_ids": ["ch1:s1:f1"],
                    "reference_image_ids": [59],
                    "image_id": 88,
                    "scene_summary": "窗边起点",
                    "frame_prompt": "角色站在窗边"
                }]
            }),
            &json!({
                "segment_number": 1,
                "source_span_ids": ["ch1:s1:f1"],
                "reference_image_ids": [59],
                "key_source_span_ids": ["ch1:s1:f1"],
                "source_span_range": {"start": "ch1:s1:f1", "end": "ch1:s1:f1"},
                "duration_secs": 5,
                "scene_summary": "角色在教室窗边回头。",
                "emotional_arc": {"end": "角色停在窗边"},
                "material_coverage": {"required_entities": ["角色"], "missing_or_weak_references": [], "risk_note": "已有素材"}
            }),
            Some(&json!({
                "shot_number": 1,
                "title": "上一段",
                "scene_summary": "上一段结尾",
                "dialogue_beats": [{
                    "speaker": "角色",
                    "line": "等等",
                    "delivery": "低声",
                    "visible_action": "嘴唇微动",
                    "listener_reaction": "对方停顿",
                    "timing": "结尾",
                    "source_span_ids": ["ch1:s1:f1"],
                    "dialogue_internal_note": "旧版额外字段"
                }],
                "reference_binding": {"bindings": [{"image_id": 59}]}
            })),
            &json!([]),
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            5,
            "16:9",
            &json!([]),
        ))
    .expect("boundary video user prompt should render");
    let payload = video_user_payload(&user);

    for key in [
        "source_span_id",
        "source_span_ids",
        "key_source_span_ids",
        "source_span_range",
        "reference_image_ids",
        "reference_binding",
        "image_id",
        "dialogue_beats",
        "dialogue_internal_note",
    ] {
        assert!(
            !contains_json_key(&payload, key),
            "final video prompt payload must not expose internal key {key}"
        );
    }
    assert!(
        !user.contains("source_span_id"),
        "final video prompt text must not expose source_span field names"
    );
    assert!(
        contains_json_key(&payload, "dialogue_cues"),
        "final video prompt payload should expose dialogue_cues as the public model field"
    );
}

#[test]
fn video_prompt_user_rejects_segment_without_number() {
    let engine = prompt_renderer();
    let err = build_boundary_video_prompt_user(final_video_context!(
        &engine,
        &json!({"segments": []}),
        &json!({"frames": []}),
        &json!({"title": "缺少片段号"}),
        None,
        &json!([]),
        "电影感写实",
        PromptOutputLanguage::ZhCn,
        5,
        "16:9",
        &json!([]),
    ))
    .expect_err("missing segment_number must fail before LLM request construction");

    assert!(
        format!("{err}").contains("缺少有效 segment_number"),
        "unexpected error: {err}"
    );
}
