#[tokio::test]
async fn boundary_video_prompt_preview_without_predecessors_returns_placeholder_chain() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let preview = service
        .preview_chapter_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(
        step_states(&preview),
        vec!["ready", "preview_placeholder", "preview_placeholder"]
    );
    assert_displayable_steps_have_requests(&preview);
    let video_step = preview_step(&preview, "video_prompt");
    assert!(
        video_step
            .user_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("chapter_video_frame_prompts")
    );
}

#[tokio::test]
async fn boundary_video_prompt_preview_with_segment_but_missing_frames_shows_frame_request() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;

    let preview = service
        .preview_chapter_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(
        step_states(&preview),
        vec!["reused", "ready", "preview_placeholder"]
    );
    assert_displayable_steps_have_requests(&preview);
    assert_eq!(preview_step(&preview, "frame_prompts").state, "ready");
    assert_incremental_boundary_frame_request(preview_step(&preview, "frame_prompts"), 1);
    assert_eq!(
        preview_step(&preview, "video_prompt").state,
        "preview_placeholder"
    );
}

#[tokio::test]
async fn chat_video_prompt_preview_without_predecessors_uses_chat_prompt_chain() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let preview = service
        .preview_chapter_chat_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(preview.workflow, "chapter_chat_video_prompt_generation");
    assert_eq!(
        preview
            .steps
            .iter()
            .map(|step| step.prompt_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "chapter_video_segment_plan",
            "chapter_video_frame_prompts",
            "chapter_video_chat_prompt"
        ]
    );
    assert_eq!(
        step_states(&preview),
        vec!["ready", "preview_placeholder", "preview_placeholder"]
    );
    assert_displayable_steps_have_requests(&preview);
}

#[tokio::test]
async fn chat_video_prompt_preview_ignores_boundary_segment_plan_when_available() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;

    let preview = service
        .preview_chapter_chat_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(preview.workflow, "chapter_chat_video_prompt_generation");
    assert_eq!(
        step_states(&preview),
        vec!["reused", "ready", "preview_placeholder"]
    );
    assert_eq!(
        preview_step(&preview, "video_prompt").prompt_name,
        "chapter_video_chat_prompt"
    );
    assert_displayable_steps_have_requests(&preview);
}
