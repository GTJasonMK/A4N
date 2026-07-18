#[tokio::test]
async fn chat_video_prompt_regeneration_uses_chat_frames_with_chat_segment_plan() {
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
    let boundary_segment_plan = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("boundary segment plan should exist");
    let mut boundary_segment_plan: chapter_video_prompt::ActiveModel = boundary_segment_plan.into();
    boundary_segment_plan.status = Set("stale".to_string());
    boundary_segment_plan.update(db.as_ref()).await.unwrap();
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service, &llm_switch, project.id, chapter.id, &engine,
    )
    .await;

    service
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

    let regenerate_llm = CapturingSequenceLlmProvider::new_owned(serial_chat_video_shot_replies());
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &regenerate_llm,
                &engine,
                regenerate_generation_options(),
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
    assert!(video_rows.iter().all(|row| {
        row.metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source_prompt_type"))
            .and_then(serde_json::Value::as_str)
            == Some("frame_segment_plan")
    }));
    assert!(
        video_rows[0]
            .visual_prompt
            .contains("小宦官站在紧闭宫门前的走廊中央偏左")
    );
    assert!(
        video_rows[0]
            .visual_prompt
            .contains("小宦官已经进入昏暗殿内并停在中央偏左位置")
    );

    let user_messages = regenerate_llm.captured_user_messages();
    let first_prompt_user = user_messages
        .iter()
        .find(|message| {
            message.contains("\"name\": \"chapter_video_chat_prompt\"")
                && message.contains("\"segment_number\": 1")
        })
        .expect("regeneration should send first chat prompt request");
    assert!(first_prompt_user.contains("\"input_frames\""));
    assert!(first_prompt_user.contains("\"start_frame\""));
    assert!(first_prompt_user.contains("\"end_frame\""));
    assert!(first_prompt_user.contains("小宦官站在紧闭宫门前的走廊中央偏左"));
    assert!(first_prompt_user.contains("小宦官已经进入昏暗殿内并停在中央偏左位置"));
}

#[tokio::test]
async fn chat_video_prompt_regeneration_respects_batch_size_and_preserves_other_prompts() {
    let db = Arc::new(setup_db().await);
    let llm_switch = SwitchableLlmProvider::new();
    let service = crate::video::test_video_orchestrator_with_llm(db.clone(), Arc::new(llm_switch.clone())).await;
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_chat_video_frame_prompt_stage(&service, &llm_switch, project.id, chapter.id, &engine).await;

    let initial_rows = service
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
    let initial_second_prompt = initial_rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 2)
        .expect("second Chat video prompt should exist");

    let mut options = video_generation_options_with_batch_size(1);
    options.regenerate_existing = true;
    let regenerate_llm =
        CapturingSequenceLlmProvider::new_owned(vec![regenerated_chat_video_shot_json(1)]);
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                &llm_switch,
                project.id,
                chapter.id,
                "chat_video",
                &regenerate_llm,
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(
        regenerate_llm.captured_user_messages().len(),
        1,
        "batch size 1 should call the main LLM once for one Chat prompt"
    );
    let regenerated_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("first Chat video prompt should exist");
    assert!(
        regenerated_prompt
            .visual_prompt
            .contains("重新生成后的 Chat 视频提示词 1")
    );
    let preserved_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 2)
        .expect("second Chat video prompt should still exist");
    assert_eq!(
        preserved_prompt.shot_number,
        initial_second_prompt.shot_number
    );
    assert!(
        !preserved_prompt
            .visual_prompt
            .contains("重新生成后的 Chat 视频提示词")
    );
}
