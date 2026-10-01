#[tokio::test]
async fn frame_card_generation_cancels_without_writing_partial_prompts() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_shared_segment_plan_stage(&service, &llm_switch, project.id, chapter.id, &engine).await;
    set_segment_recipe_mode(
        &service,
        project.id,
        chapter.id,
        1,
        a4n_video_contract::VideoSegmentWorkflowModeV1::BoundaryFrames,
        a4n_video_contract::VideoSegmentGuidanceModeV1::SystemFrames,
    )
    .await;
    let cancel_token = CancellationToken::new();
    let llm = CancellingLlmProvider::new(cancel_token.clone());
    let options = target_generation_options(&service, project.id, chapter.id, 1).await;

    let error = tokio::time::timeout(
        Duration::from_secs(2),
        service.generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &llm,
                &engine,
                options,
                Some(&cancel_token),
            ),
            None,
            None,
        ),
    )
    .await
    .expect("cancelled LLM call should return promptly")
    .expect_err("cancelled LLM call should fail");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn boundary_frame_card_and_final_prompt_share_one_explicit_segment_identity() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_shared_segment_plan_stage_with_explicit_end_frame(
        db.as_ref(),
        &service,
        &llm_switch,
        project.id,
        chapter.id,
        &engine,
    )
    .await;
    set_segment_recipe_mode(
        &service,
        project.id,
        chapter.id,
        1,
        a4n_video_contract::VideoSegmentWorkflowModeV1::BoundaryFrames,
        a4n_video_contract::VideoSegmentGuidanceModeV1::SystemFrames,
    )
    .await;
    let options = target_generation_options(&service, project.id, chapter.id, 1).await;
    let mut frame_options = options.clone();
    frame_options.prompt_batch_size = 2;

    let frame_rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(single_segment_frame_prompt_replies()),
                &engine,
                frame_options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        frame_rows
            .iter()
            .filter(|row| row.prompt_type == "frame_prompt")
            .count(),
        2
    );

    insert_boundary_frame_images(db.as_ref(), project.id, chapter.id).await;
    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt")
        .expect("target video prompt should exist");
    let identity = prompt
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("video_execution_identity"))
        .expect("target video prompt should include execution identity");
    assert_eq!(identity["source_shot_id"], json!("shot-1"));
    assert_eq!(identity["segment_index_in_shot"], json!(1));
    assert_eq!(prompt.shot_number, 1);
}

#[tokio::test]
async fn chat_video_card_generates_without_a_frame_stage() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_shared_segment_plan_stage(&service, &llm_switch, project.id, chapter.id, &engine).await;
    let options = target_generation_options(&service, project.id, chapter.id, 1).await;

    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &SequenceLlmProvider::new_owned(vec![chat_video_shot_json(1)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt")
        .expect("target Chat prompt should exist");
    assert_eq!(prompt.workflow_mode, "chat_video");
    assert!(prompt.motion_prompt.is_none());
    assert!(prompt.visual_prompt.contains("[镜头分镜]"));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::PromptType.is_in([
                "frame_prompt",
                "first_frame_prompt",
                "chat_video_frame_prompt",
            ]))
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn explicit_regeneration_rejects_a_prompt_with_generated_video() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_shared_segment_plan_stage(&service, &llm_switch, project.id, chapter.id, &engine).await;
    let options = target_generation_options(&service, project.id, chapter.id, 1).await;
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &SequenceLlmProvider::new_owned(vec![chat_video_shot_json(1)]),
                &engine,
                options.clone(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let prompt = rows
        .into_iter()
        .find(|row| row.prompt_type == "video_prompt")
        .unwrap();
    let model = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("generated prompt should still exist");
    let mut active: chapter_video_prompt::ActiveModel = model.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();
    let mut regenerate = options;
    regenerate.regenerate_existing = true;

    let error = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                regenerate,
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("generated video ownership must block prompt regeneration");

    assert!(matches!(
        error,
        AppError::InvalidArgument(message) if message.contains("不能重新生成视频提示词")
    ));
}
