#[tokio::test]
async fn first_frame_video_prompt_preview_without_predecessors_returns_placeholder_chain() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let preview = service
        .preview_chapter_first_frame_video_prompt_requests(
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
    let first_frame_step = preview_step(&preview, "first_frame_prompts");
    assert_eq!(
        first_frame_step.prompt_name,
        "chapter_video_first_frame_prompts"
    );
    let video_step = preview_step(&preview, "video_prompt");
    assert_eq!(
        video_step.prompt_name,
        "chapter_video_first_frame_video_prompt"
    );
    assert!(
        video_step
            .user_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("chapter_video_first_frame_prompts")
    );
}

#[tokio::test]
async fn first_frame_video_prompt_preview_with_segment_but_missing_initial_frame_shows_frame_request()
 {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::FirstFrameChain,
    )
    .await;

    let preview = service
        .preview_chapter_first_frame_video_prompt_requests(
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
    assert_eq!(preview_step(&preview, "first_frame_prompts").state, "ready");
    assert_eq!(
        preview_step(&preview, "video_prompt").state,
        "preview_placeholder"
    );
}
