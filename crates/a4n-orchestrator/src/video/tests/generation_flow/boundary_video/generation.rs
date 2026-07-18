#[tokio::test]
async fn video_prompt_generation_writes_segment_aligned_video_prompts_after_boundary_frames() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service = crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone())).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());

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

    let stored_frame_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_frame_count, 3);
    let stored_video_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_video_count, 0);

    let video_llm = SequenceLlmProvider::new_owned(serial_video_shot_replies());
    let mut progress_events = Vec::new();
    let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
        progress_events.push((current, total, message.to_string()));
        Ok(())
    };
    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &video_llm,
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
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        2
    );
    assert_eq!(rows[0].shot_number, 1);
    assert_eq!(rows[0].status, "pending");

    let metadata = rows[0]
        .metadata_json
        .as_ref()
        .expect("video prompt should include metadata");
    assert_eq!(
        metadata
            .get("source_prompt_type")
            .and_then(serde_json::Value::as_str),
        Some("frame_segment_plan")
    );
    assert!(metadata.get("start_boundary_frame").is_some());
    assert!(metadata.get("end_boundary_frame").is_some());
    assert!(metadata.get("self_contained_context").is_some());
    assert!(metadata.get("blocking_plan").is_some());
    assert!(metadata.get("continuity_check").is_some());
    assert!(metadata.get("temporal_beats").is_some());
    assert!(metadata.get("reference_binding").is_some());
    assert!(metadata.get("dialogue_beats").is_some());
    assert!(metadata.get("draft_chapter_video_prompt").is_none());
    assert!(metadata.get("reviewed_chapter_video_prompt").is_none());
    assert!(metadata.get("review_raw").is_none());
    let shot = metadata
        .get("shot")
        .and_then(serde_json::Value::as_object)
        .expect("metadata should include compact shot identity");
    assert!(shot.get("visual_prompt").is_none());
    assert!(shot.get("motion_prompt").is_none());
    assert!(shot.get("negative_prompt").is_none());
    assert!(progress_events.iter().any(|(_, total, message)| {
        *total == 100 && message.contains("正在逐段生成视频提示词")
    }));
}

#[tokio::test]
async fn video_prompt_generation_accepts_partial_continuous_boundary_frames() {
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

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1), video_shot_json(1)]),
                &engine,
                frame_generation_options(),
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
    assert_eq!(video_rows.len(), 1);
    assert_eq!(video_rows[0].shot_number, 1);
    let stored_video_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("boundary_frames"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_video_count, 1);
}
