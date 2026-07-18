#[tokio::test]
async fn frame_prompt_resume_rejects_segment_plan_without_source_hash() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let first_llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &first_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let segment_before = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should exist after first generation");
    assert!(
        segment_before
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .and_then(|source| source.get("duration_secs_options"))
            .is_none(),
        "source metadata must not store duration candidate options"
    );
    let mut legacy_metadata = segment_before.metadata_json.clone().unwrap();
    if let Some(source) = legacy_metadata
        .get_mut("source")
        .and_then(serde_json::Value::as_object_mut)
    {
        source.remove("content_sha256");
    }
    let mut active: chapter_video_prompt::ActiveModel = segment_before.clone().into();
    active.metadata_json = Set(Some(legacy_metadata));
    active.update(db.as_ref()).await.unwrap();

    let resume_llm = SequenceLlmProvider::new(Vec::new());
    let err = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &resume_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("segment plan without source hash must not bypass current cache identity");
    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("source.content_sha256")
                && message.contains("删除旧规划")
    ));
}

#[tokio::test]
async fn frame_prompt_resume_rejects_segment_plan_with_different_style() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_generation_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let _segment_before = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should exist after first generation");

    let mut changed_options = frame_generation_options();
    changed_options.style = "完全不同的测试风格".to_string();
    let err = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_prompt_replies()),
                &engine,
                changed_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("segment plan with a different style must not be silently retargeted");
    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("style 与当前请求不一致")
                && message.contains("删除旧规划")
    ));
}
