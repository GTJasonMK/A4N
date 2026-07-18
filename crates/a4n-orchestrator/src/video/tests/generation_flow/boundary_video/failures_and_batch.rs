#[tokio::test]
async fn video_prompt_generation_reports_missing_boundary_frame_when_partial_chain_is_exhausted() {
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
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(3))
        .exec(db.as_ref())
        .await
        .unwrap();

    service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1), video_shot_json(1)]),
                &engine,
                video_generation_options_with_batch_size(1),
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
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = first_prompt.into();
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
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                video_generation_options_with_batch_size(2),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("exhausted partial boundary chain must explain the missing frame");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("缺少边界帧提示词 #3")
                && message.contains("下一条视频提示词 #2")
    ));
}

#[tokio::test]
async fn boundary_video_prompt_generation_respects_batch_size_and_preserves_existing_prompts() {
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

    let batch_one_options = video_generation_options_with_batch_size(1);
    let first_rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1), video_shot_json(1)]),
                &engine,
                batch_one_options.clone(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let first_video_rows = first_rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(first_video_rows.len(), 1);
    assert_eq!(first_video_rows[0].shot_number, 1);

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(2), video_shot_json(2)]),
                &engine,
                batch_one_options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);
    assert_eq!(video_rows[0].shot_number, 1);
    assert_eq!(video_rows[1].shot_number, 2);
    let stored_video_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("boundary_frames"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_video_count, 2);
}
