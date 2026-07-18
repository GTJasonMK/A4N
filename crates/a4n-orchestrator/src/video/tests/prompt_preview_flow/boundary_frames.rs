#[tokio::test]
async fn boundary_frame_prompt_preview_requires_current_shooting_script() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let err = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .expect_err("preview must require an explicitly generated shooting script");

    assert!(matches!(
        err,
        AppError::NotFound(message)
            if message.contains("尚未生成当前来源版本的结构化拍摄剧本")
                && message.contains("正文页")
                && message.contains("剧本改编")
    ));
}

#[tokio::test]
async fn boundary_previews_ignore_existing_running_video_prompt() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(Arc::clone(&db)).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_matching_video_prompt(
        db.as_ref(),
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
        "running",
    )
    .await;

    let frame_preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();
    assert_displayable_steps_have_requests(&frame_preview);

    let video_preview = service
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
    assert_displayable_steps_have_requests(&video_preview);
}

#[tokio::test]
async fn boundary_frame_prompt_preview_with_segment_uses_incremental_single_frame_request() {
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
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(step_states(&preview), vec!["reused", "ready"]);
    assert_displayable_steps_have_requests(&preview);
    assert_incremental_boundary_frame_request(preview_step(&preview, "frame_prompts"), 1);
}

#[tokio::test]
async fn boundary_frame_prompt_preview_after_first_frame_targets_second_frame_with_previous_context()
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
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;
    store_boundary_frame_prompt(&service, project.id, &chapter, 1, vec![1]).await;

    let preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    let frame_step = preview_step(&preview, "frame_prompts");
    assert_incremental_boundary_frame_request(frame_step, 2);
    let user_prompt = frame_step.user_prompt.as_deref().unwrap_or_default();
    assert!(user_prompt.contains("previous_frame"));
    assert!(user_prompt.contains("边界帧 #1"));
    assert!(user_prompt.contains("\"required_segment_numbers\""));
}

#[tokio::test]
async fn boundary_frame_prompt_preview_includes_previous_frame_image_when_available() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(Arc::clone(&db)).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;
    let prompt_id = store_boundary_frame_prompt(&service, project.id, &chapter, 1, vec![1]).await;
    store_boundary_frame_image(db.as_ref(), project.id, chapter.id, prompt_id).await;

    let preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    let frame_step = preview_step(&preview, "frame_prompts");
    assert_incremental_boundary_frame_request(frame_step, 2);
    let user_prompt = frame_step.user_prompt.as_deref().unwrap_or_default();
    assert!(user_prompt.contains("previous_frame_image"));
    assert!(user_prompt.contains("boundary_frame_continuity"));
    assert!(user_prompt.contains("已生成的上一张边界帧画面"));
}

#[tokio::test]
async fn boundary_frame_preview_does_not_reuse_changed_execution_identity() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(Arc::clone(&db)).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;
    let prompt_id = store_boundary_frame_prompt(&service, project.id, &chapter, 1, vec![1]).await;
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("stored frame prompt");
    let mut metadata = prompt.metadata_json.expect("frame metadata");
    metadata["video_execution_identity"]["unit_plan_sha256"] = json!("stale-unit-plan");
    chapter_video_prompt::ActiveModel {
        id: Set(prompt_id),
        metadata_json: Set(Some(metadata)),
        ..Default::default()
    }
    .update(db.as_ref())
    .await
    .unwrap();

    let preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_incremental_boundary_frame_request(preview_step(&preview, "frame_prompts"), 1);
}

#[tokio::test]
async fn first_frame_previews_ignore_existing_running_video_prompt() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(Arc::clone(&db)).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_matching_video_prompt(
        db.as_ref(),
        project.id,
        &chapter,
        PromptWorkflowMode::FirstFrameChain,
        "running",
    )
    .await;

    let frame_preview = service
        .preview_chapter_first_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();
    assert_displayable_steps_have_requests(&frame_preview);

    let video_preview = service
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
    assert_displayable_steps_have_requests(&video_preview);
}
