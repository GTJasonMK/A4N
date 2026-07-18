#[tokio::test]
async fn video_prompt_regeneration_overwrites_existing_pending_prompts() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service = crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone())).await;
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
    let initial_rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(serial_video_shot_replies()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let initial_prompt = initial_rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("initial video prompt should exist");
    assert!(
        !initial_prompt.visual_prompt.contains("重新生成后的镜头"),
        "initial generation should use the original LLM response"
    );

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(regenerated_video_shot_replies()),
                &engine,
                regenerate_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let regenerated_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("regenerated video prompt should exist");
    assert_eq!(regenerated_prompt.id, initial_prompt.id);
    assert!(
        regenerated_prompt
            .visual_prompt
            .contains("重新生成后的镜头 1 视觉提示词"),
        "explicit regeneration should skip reusable pending video prompts"
    );
    assert_eq!(regenerated_prompt.generated_video_id, None);
    assert_eq!(regenerated_prompt.status, "pending");
}

#[tokio::test]
async fn video_prompt_regeneration_rejects_prompt_that_already_generated_video() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service = crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone())).await;
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
    service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(serial_video_shot_replies()),
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
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let err = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(regenerated_video_shot_replies()),
                &engine,
                regenerate_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("regeneration should reject prompts with generated videos");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message.contains("不能重新生成视频提示词")
    ));
}
