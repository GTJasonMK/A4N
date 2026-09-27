#[test]
fn frame_prompt_builders_require_sectioned_image_prompt_text() {
    let segment_plan =
        json!({"segments": [{"segment_number": 1, "scene_summary": "角色推门进入暗室。"}]});
    let available_assets = json!([]);
    let reference_ids = json!([]);
    let engine = prompt_engine();

    let single_frame_prompt = super::super::prompts::context::build_single_frame_prompt_user(
        frame_prompt_context!(
            &engine,
            &segment_plan,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &reference_ids,
        ),
        &json!({
            "frame_number": 2,
            "expected_segment_numbers": [1, 2],
            "required_duration_secs": 5,
            "required_aspect_ratio": "16:9",
            "previous_frame": {
                "scene_summary": "角色在门外",
                "blocking_state": {"position": "门外"},
                "lighting_state": {"key_light": "月光"},
                "frame_prompt": "角色站在门外。"
            }
        }),
    )
    .expect("single boundary frame prompt builder should render");
    assert!(single_frame_prompt.contains("【参考图约束】"));
    assert!(single_frame_prompt.contains("【构图光线风格】"));
    assert!(single_frame_prompt.contains("U 个视频执行单元共享 U+1 张边界帧"));
    assert!(single_frame_prompt.contains("constraints.reference_slots"));
    assert!(single_frame_prompt.contains("不得使用紧贴数字的参考图/图片编号写法"));
    assert!(single_frame_prompt.contains("required_frame_count"));
    assert!(single_frame_prompt.contains("focused_chapter_video_segment_plan"));
    assert!(!single_frame_prompt.contains("completed_chapter_video_frame_prompts"));
    assert!(single_frame_prompt.contains("current_frame"));
    assert!(single_frame_prompt.contains("只输出 frames 数组中的 1 条"));
    assert!(single_frame_prompt.contains("previous_frame"));
    assert!(!single_frame_prompt.contains("style_template_verbatim"));
    assert!(single_frame_prompt.contains("constraints.style"));
    assert!(!single_frame_prompt.contains("frame_number_rule"));
    assert!(!single_frame_prompt.contains("visual_prompt_sections"));
    assert!(!single_frame_prompt.contains("motion_prompt_sections"));
    assert_eq!(
        single_frame_prompt
            .matches(PromptOutputLanguage::ZhCn.instruction())
            .count(),
        1
    );
    assert!(single_frame_prompt.contains("frame_prompt 至少 80 个字符"));
    assert!(single_frame_prompt.contains("negative_prompt 至少 20 个字符"));

    let initial_frame_prompt =
        super::super::prompts::context::build_first_frame_prompt_user(frame_prompt_context!(
            &engine,
            &segment_plan,
            &available_assets,
            "电影感写实",
            PromptOutputLanguage::ZhCn,
            "16:9",
            &reference_ids,
        ))
        .expect("initial first-frame prompt builder should render");
    assert!(initial_frame_prompt.contains("【参考图约束】"));
    assert!(initial_frame_prompt.contains("【一致性与避免项】"));
    assert!(!initial_frame_prompt.contains("style_template_verbatim"));
    assert!(initial_frame_prompt.contains("电影感写实"));
    assert!(initial_frame_prompt.contains("不要自行精简、概括"));
}

#[test]
fn planning_prompt_builders_include_safe_visual_rewrite_rule() {
    let engine = prompt_engine();
    let context = super::super::prompts::context::VideoPlanningPromptContext {
        prompt_renderer: &engine,
        project_id: PROMPT_PROJECT_ID,
        chapter_summary: None,
        style: "电影感写实",
    };
    let current_shot = crate::video::tests::test_video_execution_shot(5);
    let segment_prompt =
        super::super::prompts::context::build_segment_plan_for_shot_user(context, &current_shot)
            .expect("segment plan prompt builder should render");
    assert!(segment_prompt.contains("safe_visual_rewrite_rule"));
    assert!(segment_prompt.contains("非血腥痕迹"));
    assert!(segment_prompt.contains("video_intent.hook"));
}
