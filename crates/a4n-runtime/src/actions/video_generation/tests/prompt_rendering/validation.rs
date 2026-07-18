#[test]
fn chapter_video_negative_prompt_keeps_only_explicit_specific_terms() {
    let specific = provider_specific_negative_prompt(Some("手机屏幕反光过强"))
        .expect("specific negative prompt should be preserved");

    assert_eq!(specific, "手机屏幕反光过强");
    assert_eq!(provider_specific_negative_prompt(None), None);
    assert_eq!(
        provider_specific_negative_prompt(Some("  无特殊要求  ")),
        None
    );
}

#[test]
fn provider_prompt_rejects_short_visual_prompt() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = "宽镜头，电影感".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("short provider prompt must be rejected");

    assert!(err.contains("visual_prompt 过短"));
}

#[test]
fn provider_prompt_rejects_missing_motion_prompt_without_empty_default() {
    let mut row = prompt_with_metadata();
    row.motion_prompt = None;

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("true provider prompt must require motion prompt");

    assert!(err.contains("缺少 motion_prompt"));
}

#[test]
fn provider_prompt_rejects_dictionary_like_prompt_text() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} {{\"blocking_plan\":\"站位数据不应该作为画面提示词发送\"}}",
        LONG_VISUAL
    );

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("dictionary-like prompt text should be rejected");

    assert!(err.contains("visual_prompt 包含 JSON/字典结构片段"));
}

#[test]
fn provider_prompt_rejects_internal_image_id_text_after_manual_edit() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!("{LONG_VISUAL}\n参考图 image_id=11 用于人物一致性。");

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("provider prompt must reject internal image ids in editable text");

    assert!(err.contains("visual_prompt"));
    assert!(err.contains("内部图片 ID"));
}

#[test]
fn provider_prompt_rejects_internal_workflow_prompt_text() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} 这是首帧串联模式的单段视频，起始帧来自上一段尾帧自动采集。",
        LONG_VISUAL
    );

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("internal workflow prompt text should be rejected");

    assert!(err.contains("visual_prompt 包含视频生成内部流程说明"));
    assert!(err.contains("首帧串联"));
}

#[test]
fn provider_prompt_rejects_missing_aspect_ratio_without_unspecified_default() {
    let mut row = prompt_with_metadata();
    row.aspect_ratio = None;

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect_err("true provider prompt must require aspect ratio");

    assert!(err.contains("缺少 aspect_ratio"));
}
