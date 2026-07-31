#[tokio::test]
async fn frame_prompt_generation_rejects_locked_incomplete_frames_before_llm() {
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
    insert_boundary_frame_images(db.as_ref(), project.id, chapter.id).await;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(3))
        .exec(db.as_ref())
        .await
        .unwrap();

    let err = service
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
        .expect_err("locked incomplete frame prompts must block frame LLM");

    assert!(
        matches!(
            err,
        AppError::InvalidArgument(ref message)
                if message.contains("边界帧图片")
                    && (message.contains("不能重新生成边界帧提示词")
                        || message.contains("不能删除边界帧提示词"))
        ),
        "unexpected locked-frame error: {err:?}"
    );
}

#[tokio::test]
async fn frame_prompt_generation_reuses_complete_boundary_frames_without_llm_call() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let first_llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &first_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let no_llm_calls = SequenceLlmProvider::new(Vec::new());
    let mut progress_events = Vec::new();
    let mut progress = |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
        progress_events.push((update.current, update.total, update.message));
        Ok(())
    };
    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &no_llm_calls,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            Some(&mut progress),
        )
        .await
        .unwrap();

    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "frame_prompt")
            .count(),
        3
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        0
    );
    assert!(
        progress_events
            .iter()
            .any(|(_, _, message)| message.contains("已复用 3 条边界帧提示词"))
    );
}
