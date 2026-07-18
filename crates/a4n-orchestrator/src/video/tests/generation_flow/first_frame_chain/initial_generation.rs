#[tokio::test]
async fn first_frame_chain_writes_only_initial_first_frame_then_segment_video_prompts() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service =
        crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone()))
            .await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_frame_llm = SequenceLlmProvider::new_owned(vec![
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
        frame_prompt_json(1),
    ]);
    let mut first_frame_progress_events = Vec::new();
    let mut first_frame_progress =
        |current: u32, total: u32, message: &str| -> Result<(), AppError> {
            first_frame_progress_events.push((current, total, message.to_string()));
            Ok(())
        };

    let frame_rows = service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &first_frame_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            Some(&mut first_frame_progress),
        )
        .await
        .unwrap();

    let first_frame_rows = frame_rows
        .iter()
        .filter(|row| row.prompt_type == "first_frame_prompt")
        .collect::<Vec<_>>();
    assert_eq!(first_frame_rows.len(), 1);
    assert_eq!(first_frame_rows[0].shot_number, 1);
    assert_eq!(first_frame_rows[0].workflow_mode, "first_frame_chain");
    assert!(
        first_frame_rows[0]
            .metadata_json
            .as_ref()
            .is_some_and(|metadata| metadata.get("workflow_mode").is_none()),
        "prompt metadata must not duplicate the workflow_mode column"
    );
    assert!(
        first_frame_progress_events
            .iter()
            .any(|(_, _, message)| message.contains("初始首帧提示词记录已更新"))
    );

    let video_llm = SequenceLlmProvider::new_owned(serial_video_shot_replies());
    let video_rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "first_frame_chain",
                &video_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let video_rows = video_rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);

    let first_metadata = video_rows[0]
        .metadata_json
        .as_ref()
        .expect("first shot metadata should exist");
    assert!(
        first_metadata.get("workflow_mode").is_none(),
        "video prompt metadata must not duplicate the workflow_mode column"
    );
    assert!(
        first_metadata
            .get("start_first_frame")
            .is_some_and(|value| value.is_object())
    );
    assert_eq!(
        first_metadata
            .get("first_frame_chain")
            .and_then(|value| value.get("start_source"))
            .and_then(serde_json::Value::as_str),
        Some("generated_first_frame")
    );
    assert_eq!(
        first_metadata
            .get("first_frame_chain")
            .and_then(|value| value.get("first_frame_prompt_number"))
            .and_then(serde_json::Value::as_i64),
        Some(1)
    );
    assert!(
        first_metadata
            .get("first_frame_chain")
            .and_then(|value| value.get("generated_first_frame"))
            .is_none(),
        "initial first frame should be stored once in start_first_frame, not duplicated per chain metadata"
    );
    assert!(first_metadata.get("draft_chapter_video_prompt").is_none());
    assert!(
        first_metadata
            .get("reviewed_chapter_video_prompt")
            .is_none()
    );
    assert!(first_metadata.get("review_raw").is_none());

    let second_metadata = video_rows[1]
        .metadata_json
        .as_ref()
        .expect("second shot metadata should exist");
    assert!(
        second_metadata.get("workflow_mode").is_none(),
        "video prompt metadata must not duplicate the workflow_mode column"
    );
    assert!(
        second_metadata
            .get("start_first_frame")
            .is_some_and(serde_json::Value::is_null)
    );
    assert_eq!(
        second_metadata
            .get("first_frame_chain")
            .and_then(|value| value.get("start_source"))
            .and_then(serde_json::Value::as_str),
        Some("previous_video_tail_frame")
    );
    assert!(
        second_metadata
            .get("first_frame_chain")
            .and_then(|value| value.get("first_frame_prompt_number"))
            .is_some_and(serde_json::Value::is_null)
    );
    assert_eq!(
        second_metadata
            .get("first_frame_chain")
            .and_then(|value| value.get("previous_video_shot_number"))
            .and_then(serde_json::Value::as_i64),
        Some(1)
    );
    assert!(first_metadata.get("self_contained_context").is_some());
    assert!(first_metadata.get("start_state").is_some());
    assert!(first_metadata.get("end_state").is_some());
    assert!(first_metadata.get("transition_in").is_some());
    assert_eq!(
        second_metadata
            .get("transition_in")
            .and_then(|value| value.get("from_previous_end"))
            .and_then(serde_json::Value::as_str),
        Some("承接上一片段尾态或已存在输入首帧的站位、道具和光影")
    );
}
