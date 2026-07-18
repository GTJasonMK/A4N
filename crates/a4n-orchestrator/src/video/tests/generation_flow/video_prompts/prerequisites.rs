#[tokio::test]
async fn video_prompt_generation_requires_complete_boundary_frames() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service = crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone())).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new(Vec::new());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let err = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("video prompts require saved boundary frame prompts");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message.contains("请先生成当前章节的视频片段规划")
    ));
}

#[tokio::test]
async fn frame_prompt_generation_reuses_existing_frames_when_video_prompts_exist() {
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

    let rows = service
        .generate_chapter_boundary_frame_prompts(
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
        .expect("existing video prompts should not block reusable boundary frame stage");

    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "frame_segment_plan")
            .count(),
        0
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "frame_prompt")
            .count(),
        3
    );
}
