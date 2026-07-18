#[tokio::test]
async fn chat_video_prompt_generation_preserves_generated_prompt_when_contract_matches() {
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service,
        &llm_switch,
        project.id,
        chapter.id,
        &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &SequenceLlmProvider::new_owned(serial_chat_video_shot_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let old_chat_prompt: serde_json::Value =
        serde_json::from_str(&chat_video_shot_json(1)).unwrap();
    let old_chat_prompt = old_chat_prompt["shots"][0]["chat_prompt"]
        .as_str()
        .unwrap()
        .to_string();
    let video_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = video_prompt.into();
    active.visual_prompt = Set(old_chat_prompt);
    active.reference_image_ids_json = Set(Some(json!([])));
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let llm = CapturingSequenceLlmProvider::new_owned(Vec::new());
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("contract-matching generated Chat prompt should be preserved");

    let preserved_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("first Chat video prompt should still be returned");
    assert_eq!(preserved_prompt.generated_video_id, Some(9301));
    assert_eq!(preserved_prompt.status, "completed");
    assert!(
        llm.captured_user_messages().is_empty(),
        "matching generated prompt should be returned without any LLM request"
    );
}

#[tokio::test]
async fn chat_video_prompt_generation_rejects_nonmatching_generated_video_before_planning_llm() {
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service,
        &llm_switch,
        project.id,
        chapter.id,
        &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &SequenceLlmProvider::new_owned(serial_chat_video_shot_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let video_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.style = Set(Some("不同风格".to_string()));
    active.update(db.as_ref()).await.unwrap();

    let llm = CapturingSequenceLlmProvider::new_owned(Vec::new());
    let err = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("nonmatching generated-video-linked Chat prompt should block before planning");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message.contains("不能重新生成视频提示词")
    ));
    assert!(
        llm.captured_user_messages().is_empty(),
        "preflight should reject before shooting script projection or segment planning LLM"
    );
}

#[tokio::test]
async fn chat_video_prompt_generation_preserves_generated_reused_prompt_during_checkpoint_sync() {
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service,
        &llm_switch,
        project.id,
        chapter.id,
        &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &SequenceLlmProvider::new_owned(serial_chat_video_shot_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let first_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should exist");
    let first_prompt_id = first_prompt.id;
    let mut first_metadata = first_prompt
        .metadata_json
        .clone()
        .unwrap_or_else(|| json!({}));
    first_metadata["preserved_checkpoint_marker"] = json!(true);
    let mut active: chapter_video_prompt::ActiveModel = first_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.metadata_json = Set(Some(first_metadata.clone()));
    active.update(db.as_ref()).await.unwrap();

    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(2))
        .exec(db.as_ref())
        .await
        .unwrap();

    let llm = CapturingSequenceLlmProvider::new_owned(vec![chat_video_shot_json(2)]);
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &llm,
                &engine,
                video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("locked reused Chat prompt must be preserved while missing prompt is generated");

    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        2
    );
    assert!(
        !llm.captured_user_messages().is_empty(),
        "missing second prompt should still be generated"
    );

    let stored_first = chapter_video_prompt::Entity::find_by_id(first_prompt_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should remain");
    assert_eq!(stored_first.generated_video_id, Some(9301));
    assert_eq!(stored_first.metadata_json, Some(first_metadata));
}
