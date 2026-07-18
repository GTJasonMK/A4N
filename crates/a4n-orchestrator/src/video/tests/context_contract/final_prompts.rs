#[test]
fn video_prompt_builders_require_sectioned_api_prompt_text() {
    let current_segment = json!({"segment_number": 1, "duration_secs": 10});
    let segment_plan = json!({"segments": [current_segment.clone()]});
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let boundary_prompt =
        super::super::prompt_context::build_boundary_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("boundary prompt builder should render");
    assert!(boundary_prompt.contains("[主体定义与素材绑定]"));
    assert!(boundary_prompt.contains("[镜头分镜]"));
    assert!(boundary_prompt.contains("[对白与表演]"));
    assert!(boundary_prompt.contains("风格模板：京阿尼动画风格"));
    assert!(boundary_prompt.contains("不要配背景音乐"));

    let first_frame_prompt =
        super::super::prompt_context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("first-frame prompt builder should render");
    assert!(first_frame_prompt.contains("\"input_frame_rule\""));
    assert!(first_frame_prompt.contains("\"input_frame_usage\""));
    assert!(first_frame_prompt.contains("current_segment_end_state"));
    assert!(first_frame_prompt.contains("[镜头分镜]"));
    assert!(first_frame_prompt.contains("风格模板：京阿尼动画风格"));
    assert!(first_frame_prompt.contains("不要配背景音乐"));
}

#[test]
fn video_prompt_builders_share_final_prompt_contract() {
    let current_segment = json!({
        "segment_number": 1,
        "duration_secs": 10,
        "source_span_ids": ["ch1:s1:f1"]
    });
    let segment_plan = json!({"segments": [current_segment.clone()]});
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let boundary_prompt =
        super::super::prompt_context::build_boundary_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("boundary prompt builder should render");

    let first_frame_prompt =
        super::super::prompt_context::build_first_frame_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("first-frame prompt builder should render");

    let chat_prompt =
        super::super::prompt_context::build_chat_video_prompt_user(final_video_context!(
            &engine,
            &segment_plan,
            &json!({"frames": []}),
            &current_segment,
            None,
            &available_assets,
            "风格模板：京阿尼动画风格",
            PromptOutputLanguage::ZhCn,
            10,
            "16:9",
            &reference_ids,
        ))
        .expect("chat prompt builder should render");

    for prompt in [&boundary_prompt, &first_frame_prompt, &chat_prompt] {
        assert!(prompt.contains("\"prompt_text_section_rule\""));
        assert!(prompt.contains("\"prompt_density_rule\""));
        assert!(prompt.contains("\"safe_visual_rewrite_rule\""));
        assert!(prompt.contains("\"reference_display_label_rule\""));
        assert!(prompt.contains("\"natural_language_no_internal_fields_rule\""));
        assert!(prompt.contains("最终自然语言主提示词"));
        assert!(prompt.contains("duration_secs 内自然完成且能被观众读懂的动作"));
        assert!(prompt.contains("不要配背景音乐"));
    }
}
