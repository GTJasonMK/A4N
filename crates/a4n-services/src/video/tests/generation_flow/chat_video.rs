use super::*;

#[tokio::test]
async fn chat_video_prompt_generation_writes_copyable_prompts_after_frame_stage() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_chat_video_frame_prompt_stage(&service, project.id, chapter.id, &engine).await;
    let llm = SequenceLlmProvider::new_owned(serial_chat_video_shot_replies());
    let mut progress_events = Vec::new();
    let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
        progress_events.push((current, total, message.to_string()));
        Ok(())
    };

    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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
    assert_eq!(video_rows.len(), 3);
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
    assert_eq!(frame_stage_count, 4);

    let segment_plan_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("boundary_frames"))
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
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service, project.id, chapter.id, &engine,
    )
    .await;

    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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
    assert_eq!(video_rows.len(), 3);
    assert_eq!(video_rows[0].shot_number, 1);
    assert_eq!(video_rows[1].shot_number, 2);
    assert_eq!(video_rows[2].shot_number, 3);
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

    let chat_segment_plan_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("chat_video_segment_plan"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(chat_segment_plan_count, 0);
}

#[tokio::test]
async fn chat_video_prompt_generation_preserves_generated_prompt_when_contract_matches() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service, project.id, chapter.id, &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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

    let old_chat_prompt: serde_json::Value =
        serde_json::from_str(&chat_video_shot_json(1)).unwrap();
    let old_chat_prompt = old_chat_prompt["shots"][0]["chat_prompt"]
        .as_str()
        .unwrap()
        .to_string();
    let video_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = video_prompt.into();
    active.visual_prompt = Set(old_chat_prompt);
    active.reference_image_ids_json = Set(Some(json!([])));
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let llm = CapturingSequenceLlmProvider::new_owned(Vec::new());
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "chat_video",
                &llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("contract-matching generated Chat prompt should be preserved");

    let preserved_prompt = rows
        .iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("first Chat video prompt should still be returned");
    assert_eq!(preserved_prompt.generated_video_id, Some(9301));
    assert_eq!(preserved_prompt.status, "completed");
    assert!(
        llm.captured_user_messages().is_empty(),
        "matching generated prompt should be returned without any LLM request"
    );
}

#[tokio::test]
async fn chat_video_prompt_generation_rejects_nonmatching_generated_video_before_planning_llm() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service, project.id, chapter.id, &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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

    let video_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let mut changed_options = frame_generation_options();
    changed_options.style = "不同风格".to_string();
    let llm = CapturingSequenceLlmProvider::new_owned(Vec::new());
    let err = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "chat_video",
                &llm,
                &engine,
                changed_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("nonmatching generated-video-linked Chat prompt should block before planning");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message.contains("不能重新生成视频提示词")
    ));
    assert!(
        llm.captured_user_messages().is_empty(),
        "preflight should reject before shot analysis or segment planning LLM"
    );
}

#[tokio::test]
async fn chat_video_prompt_generation_preserves_generated_reused_prompt_during_checkpoint_sync() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
    generate_chat_video_frame_prompt_stage_from_existing_plan(
        &service, project.id, chapter.id, &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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

    let first_prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should exist");
    let first_prompt_id = first_prompt.id;
    let mut first_metadata = first_prompt
        .metadata_json
        .clone()
        .unwrap_or_else(|| json!({}));
    first_metadata["preserved_checkpoint_marker"] = json!(true);
    let mut active: chapter_video_prompt::ActiveModel = first_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.metadata_json = Set(Some(first_metadata.clone()));
    active.update(db.as_ref()).await.unwrap();

    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("chat_video"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(2))
        .exec(db.as_ref())
        .await
        .unwrap();

    let llm = CapturingSequenceLlmProvider::new_owned(vec![
        chat_video_shot_json(2),
        chat_video_shot_json(2),
    ]);
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "chat_video",
                &llm,
                &engine,
                video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("locked reused Chat prompt must be preserved while missing prompt is generated");

    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        3
    );
    assert!(
        !llm.captured_user_messages().is_empty(),
        "missing second prompt should still be generated"
    );

    let stored_first = chapter_video_prompt::Entity::find_by_id(first_prompt_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first Chat video prompt should remain");
    assert_eq!(stored_first.generated_video_id, Some(9301));
    assert_eq!(stored_first.metadata_json, Some(first_metadata));
}

#[tokio::test]
async fn chat_video_prompt_regeneration_uses_chat_frames_with_chat_segment_plan() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
        &service, project.id, chapter.id, &engine,
    )
    .await;

    service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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
    assert_eq!(video_rows.len(), 3);
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
    let first_review_user = user_messages
        .iter()
        .find(|message| {
            message.contains("\"name\": \"chapter_video_chat_prompt_review\"")
                && message.contains("\"segment_number\": 1")
        })
        .expect("regeneration should send first chat prompt review request");

    for user_prompt in [first_prompt_user, first_review_user] {
        assert!(user_prompt.contains("\"input_frames\""));
        assert!(user_prompt.contains("\"start_frame\""));
        assert!(user_prompt.contains("\"end_frame\""));
        assert!(user_prompt.contains("小宦官站在紧闭宫门前的走廊中央偏左"));
        assert!(user_prompt.contains("小宦官已经进入昏暗殿内并停在中央偏左位置"));
    }
}

#[tokio::test]
async fn chat_video_prompt_regeneration_respects_batch_size_and_preserves_other_prompts() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    generate_chat_video_frame_prompt_stage(&service, project.id, chapter.id, &engine).await;

    let initial_rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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
    let regenerate_llm = CapturingSequenceLlmProvider::new_owned(vec![
        regenerated_chat_video_shot_json(1),
        regenerated_chat_video_shot_json(1),
    ]);
    let rows = service
        .generate_chapter_chat_video_prompts(
            prompt_stage_input(
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
        2,
        "batch size 1 should call LLM only for draft and review of one Chat prompt"
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
