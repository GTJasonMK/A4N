#[tokio::test]
async fn frame_prompt_generation_cancels_while_waiting_for_llm_without_insert() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let cancel_token = CancellationToken::new();
    let llm = CancellingLlmProvider::new(cancel_token.clone());

    let err = tokio::time::timeout(
        Duration::from_secs(2),
        service.generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &llm,
                &engine,
                frame_generation_options(),
                Some(&cancel_token),
            ),
            None,
            None,
        ),
    )
    .await
    .expect("cancelled LLM call should return promptly")
    .expect_err("cancelled LLM call should fail");

    assert!(matches!(err, AppError::Cancelled));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}
