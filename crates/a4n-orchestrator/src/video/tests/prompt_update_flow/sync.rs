#[tokio::test]
async fn sync_prompts_rejects_regenerating_frame_prompt_with_generated_image() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
                "prompt_id": frame.id
            }
        })),
    )
    .await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "frame_prompt",
                shot_number: 1,
                title: Some("重生成边界帧"),
                scene_summary: Some("重生成摘要"),
                visual_prompt: "这是重新生成后的边界帧提示词，内容已经变化",
                motion_prompt: None,
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(json!({
                    "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same"}
                })),
            })],
        )
        .await
        .expect_err("prompt sync must not rewrite frame prompt after image generation");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片") && message.contains("不能重新生成边界帧提示词")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(frame.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, frame.visual_prompt);
}
