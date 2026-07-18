#[tokio::test]
async fn video_prompt_generation_reuses_matching_prompt_that_already_generated_video() {
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

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let reused_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("first video prompt should still be returned");
    assert_eq!(reused_prompt.generated_video_id, Some(9301));
    assert_eq!(reused_prompt.status, "completed");
}

#[tokio::test]
async fn video_prompt_generation_next_batch_preserves_generated_video_prompt_without_rewriting() {
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
                video_generation_options_with_batch_size(1),
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
    let first_prompt_id = video_prompt.id;
    let mut active: chapter_video_prompt::ActiveModel = video_prompt.clone().into();
    active.style = Set(Some("旧风格".to_string()));
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(2), video_shot_json(2)]),
                &engine,
                video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("next batch should preserve generated prompt and create the next prompt");

    let preserved_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("generated first video prompt should still be returned");
    assert_eq!(preserved_prompt.id, first_prompt_id);
    assert_eq!(preserved_prompt.generated_video_id, Some(9301));
    assert_eq!(preserved_prompt.status, "completed");
    assert_eq!(preserved_prompt.style.as_deref(), Some("旧风格"));

    let next_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 2)
        .expect("next video prompt should be generated");
    assert_eq!(next_prompt.generated_video_id, None);
    assert_eq!(next_prompt.status, "pending");
}
