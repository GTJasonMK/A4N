#[tokio::test]
async fn segment_plan_requires_current_shooting_script_before_llm() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), project.id, 1).await;

    let error = service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "shared_segment_plan",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("segment planning must require a current shooting script");

    assert!(matches!(
        error,
        AppError::NotFound(message)
            if message.contains("未找到与当前正文版本匹配的结构化拍摄剧本")
    ));
}

#[tokio::test]
async fn segment_plan_publishes_each_completed_shot_before_the_stage_finishes() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let cancel_token = CancellationToken::new();
    let progress_cancel = cancel_token.clone();
    let mut progress = move |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
        if update.stage_code == "video_prompt_item_committed"
            && update.item_key.as_deref() == Some("frame_segment_plan:0")
        {
            progress_cancel.cancel();
        }
        Ok(())
    };

    let error = service
        .generate_chapter_segment_plan(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "shared_segment_plan",
                &SequenceLlmProvider::new_owned(vec![
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                ]),
                &engine,
                frame_generation_options(),
                Some(&cancel_token),
            ),
            None,
            Some(&mut progress),
        )
        .await
        .expect_err("the test cancels after the first published shot");
    assert!(matches!(error, AppError::Cancelled));

    let plans = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(plans.len(), 1);
    let metadata = plans[0].metadata_json.as_ref().unwrap();
    assert_eq!(metadata["plan_complete"], json!(false));
    assert_eq!(metadata["segments"].as_array().unwrap().len(), 1);
    assert_eq!(metadata["segment_recipes"].as_array().unwrap().len(), 1);
    assert!(metadata["video_execution_identity"].is_object());
}
