#[test]
fn provider_preview_text_placeholders_keep_blank_draft_request_renderable() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = String::new();
    row.motion_prompt = None;
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview placeholders should make provider prompt renderable");

    let note = note.expect("blank preview fields should be noted");
    assert!(note.contains("visual_prompt 过短"));
    assert!(note.contains("motion_prompt 过短"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(prompt.contains(VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
}

#[test]
fn provider_preview_replaces_deprecated_motion_continuity_section_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.motion_prompt = Some("[运镜与动作]\n镜头保持平视中景轻微手持漂移，小宦官先停住听向暗角，再缓慢回头并轻微后缩，动作节奏压低且不跳轴。\n\n[对白与表演]\n小宦官压低声音说“谁在那儿”，嘴唇短促微动，随后屏住呼吸，眼神仍锁向黑暗处。\n\n[连续性要求]\n这是旧版重复章节，应该由 visual_prompt 的连续性与结尾状态承担。".to_string());
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace deprecated motion sections with a placeholder");

    let note = note.expect("deprecated preview fields should be noted");
    assert!(note.contains("motion_prompt 包含已废弃章节 [连续性要求]"));
    assert!(prompt.contains(VIDEO_PROVIDER_MOTION_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("[连续性要求]"));
    assert!(!prompt.contains("旧版重复章节"));
}

#[test]
fn provider_preview_replaces_dictionary_like_prompt_text_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} {{\"blocking_plan\":\"站位数据不应该作为画面提示词发送\"}}",
        LONG_VISUAL
    );
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace dictionary-like visual prompt");

    let note = note.expect("dictionary-like preview fields should be noted");
    assert!(note.contains("visual_prompt 包含 JSON/字典结构片段"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("blocking_plan"));
    assert!(!prompt.contains("{"));
}

#[test]
fn provider_preview_replaces_internal_workflow_prompt_text_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!(
        "{} 首帧串联规则：本段起始帧来自上一段尾帧自动采集。",
        LONG_VISUAL
    );
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace internal workflow visual prompt");

    let note = note.expect("internal workflow preview fields should be noted");
    assert!(note.contains("visual_prompt 包含视频生成内部流程说明"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("首帧串联规则"));
    assert!(!prompt.contains("上一段尾帧"));
}

#[test]
fn provider_preview_replaces_internal_image_id_prompt_text_with_placeholder() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = format!("{LONG_VISUAL}\n参考图 image_id=11 用于人物一致性。");
    let mut note = None;

    ensure_chapter_video_prompt_preview_texts(&mut row, &mut note);
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
    )
    .expect("preview should replace internal image id visual prompt");

    let note = note.expect("internal image id preview fields should be noted");
    assert!(note.contains("visual_prompt 不符合自然语言提示词约束"));
    assert!(note.contains("内部图片 ID"));
    assert!(prompt.contains(VIDEO_PROVIDER_VISUAL_PREVIEW_PLACEHOLDER));
    assert!(!prompt.contains("image_id=11"));
}

#[test]
fn provider_preview_metadata_placeholders_keep_request_renderable() {
    let mut row = prompt_with_metadata();
    row.metadata_json = None;
    let mut note = None;

    ensure_chapter_video_prompt_preview_metadata(&mut row, &mut note)
        .expect("preview metadata placeholders should be built");
    let prompt = build_video_generation_provider_preview_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: None,
            last: None,
        },
    )
    .expect("preview metadata placeholders should make provider prompt renderable");

    let note = note.expect("missing metadata should be noted");
    assert!(note.contains("缺少结构化 metadata_json"));
    assert!(note.contains("metadata.shot"));
    assert!(prompt.contains("【视频生成主提示词】"));
    assert!(prompt.contains("【动作与运镜补充】"));
    assert!(!prompt.contains("请求预览占位"));
    assert!(!prompt.contains("【首尾帧状态】"));
    assert!(!prompt.contains("【片段独立生成上下文】"));
    assert!(!prompt.contains("【连续性自检】"));
    assert!(!prompt.contains("\"blocking\""));
}

#[test]
fn provider_prompt_still_rejects_missing_metadata_without_preview_placeholders() {
    let mut row = prompt_with_metadata();
    row.metadata_json = None;

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: None,
            last: None,
        },
    )
    .expect_err("true provider prompt should still require structured metadata");

    assert!(err.contains("缺少结构化元数据"));
}

#[test]
fn provider_preview_override_applies_editor_draft_fields() {
    let mut row = prompt_with_metadata();
    let mut note = None;

    apply_chapter_video_prompt_preview_override(
        &mut row,
        Some(PreviewChapterVideoPromptOverrideInput {
            visual_prompt: Some("草稿视觉提示".to_string()),
            motion_prompt: Some(Some("草稿动作提示".to_string())),
            negative_prompt: Some(None),
            duration_secs: Some(0),
            reference_image_ids_json: Some(Some(json!([31, 32]))),
        }),
        &mut note,
    );

    assert_eq!(row.visual_prompt, "草稿视觉提示");
    assert_eq!(row.motion_prompt.as_deref(), Some("草稿动作提示"));
    assert_eq!(row.negative_prompt, None);
    assert_eq!(row.duration_secs, 1);
    assert_eq!(row.reference_image_ids_json, Some(json!([31, 32])));
    let note = note.expect("draft override should be noted");
    assert!(note.contains("草稿覆盖"));
    assert!(note.contains("duration_secs=0 无效"));
}

#[test]
fn preview_prompt_override_distinguishes_missing_and_null_nullable_fields() {
    let missing = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({}))
        .expect("empty override should deserialize");
    assert_eq!(missing.motion_prompt, None);
    assert_eq!(missing.negative_prompt, None);
    assert_eq!(missing.reference_image_ids_json, None);

    let cleared = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({
        "motion_prompt": null,
        "negative_prompt": null,
        "reference_image_ids_json": null
    }))
    .expect("null override fields should deserialize as explicit clears");
    assert_eq!(cleared.motion_prompt, Some(None));
    assert_eq!(cleared.negative_prompt, Some(None));
    assert_eq!(cleared.reference_image_ids_json, Some(None));
}
