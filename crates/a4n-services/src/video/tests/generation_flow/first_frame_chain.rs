use super::*;

#[tokio::test]
async fn first_frame_chain_writes_only_initial_first_frame_then_segment_video_prompts() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_frame_llm = SequenceLlmProvider::new_owned(vec![
        BEAT_EXTRACTION_JSON.to_string(),
        SHOT_ANALYSIS_JSON.to_string(),
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
    assert_eq!(video_rows.len(), 3);

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

#[tokio::test]
async fn first_frame_chain_frame_prompt_generation_does_not_silently_reuse_existing_prompt() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let initial_rows = service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    BEAT_EXTRACTION_JSON.to_string(),
                    SHOT_ANALYSIS_JSON.to_string(),
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                    frame_prompt_json(1),
                ]),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
    let initial_first_frame = initial_rows
        .iter()
        .find(|row| row.prompt_type == "first_frame_prompt")
        .unwrap();

    let mut regenerated_frame: serde_json::Value =
        serde_json::from_str(&frame_prompt_json(1)).unwrap();
    regenerated_frame["frames"][0]["title"] = json!("重新生成的初始首帧");
    let mut progress_events = Vec::new();
    let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
        progress_events.push((current, total, message.to_string()));
        Ok(())
    };

    let regenerated_rows = service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![regenerated_frame.to_string()]),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            Some(&mut progress),
        )
        .await
        .unwrap();

    let regenerated_first_frame = regenerated_rows
        .iter()
        .find(|row| row.prompt_type == "first_frame_prompt")
        .unwrap();
    assert_eq!(regenerated_first_frame.id, initial_first_frame.id);
    assert_eq!(
        regenerated_first_frame.title.as_deref(),
        Some("重新生成的初始首帧")
    );
    assert!(
        progress_events
            .iter()
            .any(|(_, _, message)| message.contains("正在生成初始首帧提示词"))
    );
    assert!(
        !progress_events
            .iter()
            .any(|(_, _, message)| message.contains("已复用")),
        "普通生成首帧不能静默复用旧记录，否则界面会显示完成但数量不变"
    );
}

#[tokio::test]
async fn first_frame_chain_english_video_prompt_continues_after_existing_zh_video_prompt() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let zh_video_prompt =
        store_first_frame_chain_prompt_seed(&service, project.id, chapter.id).await;
    let mut active: chapter_video_prompt::ActiveModel = zh_video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let en_video_rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    english_video_shot_json(2),
                    english_video_shot_json(2),
                ]),
                &engine,
                english_video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("English generation should continue after an existing Chinese video prompt");

    let en_video_prompt = en_video_rows
        .iter()
        .find(|row| {
            row.prompt_type == "video_prompt"
                && row.shot_number == 2
                && row.metadata_json.as_ref().is_some_and(|metadata| {
                    metadata
                        .get("prompt_output_language")
                        .and_then(serde_json::Value::as_str)
                        == Some("en")
                })
        })
        .expect("en video prompt should be saved");
    assert_eq!(en_video_prompt.generated_video_id, None);

    let stored_shot_1 = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_shot_1.len(), 1);
    assert_eq!(stored_shot_1[0].generated_video_id, Some(9301));
    assert!(
        stored_shot_1[0]
            .metadata_json
            .as_ref()
            .is_some_and(|metadata| {
                metadata
                    .get("prompt_output_language")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("zh_cn")
                    == "zh_cn"
            })
    );

    let stored_shot_2 = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(2))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(stored_shot_2.iter().any(|row| {
        row.generated_video_id.is_none()
            && row.metadata_json.as_ref().is_some_and(|metadata| {
                metadata
                    .get("prompt_output_language")
                    .and_then(serde_json::Value::as_str)
                    == Some("en")
            })
    }));
}

#[tokio::test]
async fn first_frame_chain_zh_video_prompt_continues_after_generated_zh_video_prompt() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let zh_video_prompt =
        store_first_frame_chain_prompt_seed(&service, project.id, chapter.id).await;
    let first_prompt_id = zh_video_prompt.id;
    let mut active: chapter_video_prompt::ActiveModel = zh_video_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let mut options = video_generation_options_with_batch_size(1);
    options.prompt_output_language = Some("zh_cn".to_string());
    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(2), video_shot_json(2)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect(
            "Chinese generation should continue after an existing Chinese generated video prompt",
        );

    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(video_rows.len(), 2);
    let reused_prompt = video_rows
        .iter()
        .find(|row| row.shot_number == 1)
        .expect("first prompt should be preserved");
    assert_eq!(reused_prompt.id, first_prompt_id);
    assert_eq!(reused_prompt.generated_video_id, Some(9301));
    let generated_next_prompt = video_rows
        .iter()
        .find(|row| row.shot_number == 2)
        .expect("second prompt should be generated");
    assert_eq!(generated_next_prompt.generated_video_id, None);
    assert!(
        generated_next_prompt
            .metadata_json
            .as_ref()
            .is_some_and(|metadata| {
                metadata
                    .get("prompt_output_language")
                    .and_then(serde_json::Value::as_str)
                    == Some("zh_cn")
            })
    );

    let stored_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_prompts.len(), 2);
    assert_eq!(stored_prompts[0].id, first_prompt_id);
    assert_eq!(stored_prompts[0].generated_video_id, Some(9301));
    assert_eq!(stored_prompts[1].shot_number, 2);
    assert_eq!(stored_prompts[1].generated_video_id, None);
}

#[tokio::test]
async fn first_frame_chain_video_prompt_generation_resumes_from_saved_serial_prefix() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_frame_llm = SequenceLlmProvider::new_owned(vec![
        BEAT_EXTRACTION_JSON.to_string(),
        SHOT_ANALYSIS_JSON.to_string(),
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
        frame_prompt_json(1),
    ]);

    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &first_frame_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let interrupted_llm =
        SequenceLlmProvider::new_owned(vec![video_shot_json(1), video_shot_json(1)]);
    service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &interrupted_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("second segment should fail after first segment checkpoint");

    let saved_after_interrupt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("first_frame_chain"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(saved_after_interrupt.len(), 1);
    assert_eq!(saved_after_interrupt[0].shot_number, 1);
    assert!(
        saved_after_interrupt[0]
            .metadata_json
            .as_ref()
            .is_some_and(|metadata| metadata.get("end_state").is_some())
    );

    let resume_llm = SequenceLlmProvider::new_owned(vec![
        video_shot_json(2),
        video_shot_json(2),
        video_shot_json(3),
        video_shot_json(3),
    ]);
    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &resume_llm,
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
}

#[tokio::test]
async fn first_frame_chain_video_prompt_uses_selected_duration_when_options_include_shorter_value()
{
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    BEAT_EXTRACTION_JSON.to_string(),
                    SHOT_ANALYSIS_JSON.to_string(),
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                    frame_prompt_json(1),
                ]),
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let mut video_options = frame_generation_options();
    video_options.duration_secs = 8;
    video_options.duration_secs_options = vec![4, 8];

    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(serial_video_shot_replies()),
                &engine,
                video_options,
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
    assert!(video_rows.iter().all(|row| row.duration_secs == 8));
}

#[tokio::test]
async fn first_frame_chain_video_prompt_generation_respects_batch_size() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_first_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
                &SequenceLlmProvider::new_owned(vec![
                    BEAT_EXTRACTION_JSON.to_string(),
                    SHOT_ANALYSIS_JSON.to_string(),
                    segment_plan_json_for_shot(1, None),
                    segment_plan_json_for_shot(2, None),
                    frame_prompt_json(1),
                ]),
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
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
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
    assert_eq!(
        first_rows
            .iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        1
    );

    let rows = service
        .generate_chapter_first_frame_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "first_frame_chain",
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
}
