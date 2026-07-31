#[tokio::test]
async fn chat_video_prompt_generation_writes_copyable_prompts_after_segment_plan() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_chat_video_segment_plan_stage(&service, &llm_switch, project.id, chapter.id, &engine)
        .await;
    let llm = SequenceLlmProvider::new_owned(serial_chat_video_shot_replies());
    let mut progress_events = Vec::new();
    let mut progress = |update: VideoPromptProgressUpdate| -> Result<(), AppError> {
        progress_events.push((update.current, update.total, update.message));
        Ok(())
    };

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
            Some(&mut progress),
        )
        .await
        .unwrap();

    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);
    assert!(
        video_rows
            .iter()
            .all(|row| row.workflow_mode == "chat_video")
    );
    assert!(
        video_rows
            .iter()
            .all(|row| row.motion_prompt.as_deref().is_none())
    );
    assert!(video_rows[0].visual_prompt.contains("[主体定义与素材绑定]"));
    assert!(video_rows[0].visual_prompt.contains("[镜头分镜]"));
    assert!(
        !video_rows[0].visual_prompt.contains("\"shot_number\""),
        "copied Chat prompt should be natural language, not JSON"
    );
    assert_eq!(
        video_rows[0].reference_image_ids_json.as_ref(),
        Some(&json!([]))
    );

    let metadata = video_rows[0]
        .metadata_json
        .as_ref()
        .expect("Chat video prompt should include metadata");
    assert_eq!(
        metadata
            .get("source_prompt_type")
            .and_then(serde_json::Value::as_str),
        Some("frame_segment_plan")
    );
    assert_eq!(
        metadata
            .get("contract_version")
            .and_then(serde_json::Value::as_str),
        Some(CHAT_VIDEO_CONTRACT_VERSION)
    );
    assert!(
        metadata
            .get("chat_video")
            .is_some_and(serde_json::Value::is_object)
    );
    assert_eq!(
        metadata.get("generation_reference_image_ids"),
        Some(&json!([]))
    );

    let frame_stage_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.is_in([
            "frame_prompt",
            "first_frame_prompt",
            "chat_video_frame_prompt",
        ]))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(frame_stage_count, 0);

    let segment_plan_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("shared_segment_plan"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(segment_plan_count, 1);
    assert!(
        progress_events.iter().any(|(_, total, message)| {
            *total == 100 && message.contains("Chat 视频提示词")
        })
    );
}

#[tokio::test]
async fn chat_video_prompt_generation_reuses_shared_segment_plan_when_available() {
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
    let rows = service
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

    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);
    assert_eq!(video_rows[0].shot_number, 1);
    assert_eq!(video_rows[1].shot_number, 2);
    assert!(
        video_rows
            .iter()
            .all(|row| row.workflow_mode == "chat_video")
    );
    assert!(video_rows.iter().all(|row| {
        row.metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source_prompt_type"))
            .and_then(serde_json::Value::as_str)
            == Some("frame_segment_plan")
    }));
}
