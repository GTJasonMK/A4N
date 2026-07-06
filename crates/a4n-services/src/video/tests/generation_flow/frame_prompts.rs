use super::*;

#[tokio::test]
async fn frame_prompt_generation_cancels_while_waiting_for_llm_without_insert() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let cancel_token = CancellationToken::new();
    let llm = CancellingLlmProvider {
        cancel_token: cancel_token.clone(),
    };

    let err = tokio::time::timeout(
        Duration::from_secs(2),
        service.generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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

#[tokio::test]
async fn frame_prompt_generation_writes_only_segment_plan_and_boundary_frames() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let mut progress_events = Vec::new();
    let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
        progress_events.push((current, total, message.to_string()));
        Ok(())
    };

    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project_id,
                chapter_id,
                "boundary_frames",
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
    let frame_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();

    assert_eq!(video_rows.len(), 0);
    assert_eq!(frame_rows.len(), 4);
    assert!(
        rows.iter()
            .any(|row| row.prompt_type == "frame_segment_plan")
    );
    assert!(progress_events.iter().any(|(_, total, message)| {
        *total == 100 && message.contains("边界帧提示词记录已更新")
    }));
}

#[tokio::test]
async fn video_prompt_generation_counts_boundary_frames_by_generation_context_not_frame_references()
{
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let reference_image = insert_generated_image(db.as_ref(), project.id).await;
    let mut options = frame_generation_options();
    options.reference_image_ids = vec![reference_image.id];

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_generation_replies()),
                &engine,
                options.clone(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let frame_rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(frame_rows.len(), 4);
    assert!(
        frame_rows
            .iter()
            .all(|row| row.reference_image_ids_json.as_ref() == Some(&json!([])))
    );

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(serial_video_shot_replies()),
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
        rows.iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .count(),
        3
    );
    let video_rows = rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert!(
        video_rows
            .iter()
            .all(|row| row.reference_image_ids_json.as_ref() == Some(&json!([reference_image.id]))),
        "最终视频提示词应继承本次用户选择的素材参考图，而不是依赖片段规划旧字段"
    );
}

#[tokio::test]
async fn frame_prompt_regeneration_replans_segment_for_changed_duration() {
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
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .exec(db.as_ref())
        .await
        .unwrap();
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .exec(db.as_ref())
        .await
        .unwrap();

    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned({
                    let mut replies = vec![
                        BEAT_EXTRACTION_JSON.to_string(),
                        shot_analysis_json_with_duration(5),
                        segment_plan_json_for_shot(1, Some(5)),
                        segment_plan_json_for_shot(2, Some(5)),
                    ];
                    replies.extend([
                        frame_prompt_json_with_duration(1, 5),
                        frame_prompt_json_with_duration(2, 5),
                        frame_prompt_json_with_duration(3, 5),
                        frame_prompt_json_with_duration(4, 5),
                    ]);
                    replies
                }),
                &engine,
                regenerate_generation_options_with_duration(5),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let segment_plan = rows
        .iter()
        .find(|row| row.prompt_type == "frame_segment_plan")
        .expect("regeneration should keep a segment plan row");
    assert_eq!(segment_plan.duration_secs, 5);
    let frame_prompts = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();
    assert_eq!(frame_prompts.len(), 4);
    assert!(
        frame_prompts.iter().all(|row| row.duration_secs == 5),
        "regenerated frame prompts must use the user-selected duration"
    );
}

#[tokio::test]
async fn frame_prompt_regeneration_deletes_stale_frames_when_segment_count_shrinks() {
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
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .exec(db.as_ref())
        .await
        .unwrap();
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .exec(db.as_ref())
        .await
        .unwrap();

    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned({
                    let mut replies = vec![
                        BEAT_EXTRACTION_JSON.to_string(),
                        single_shot_analysis_json(5),
                        single_segment_plan_json(5),
                    ];
                    replies.extend(single_segment_frame_prompt_replies(5));
                    replies
                }),
                &engine,
                regenerate_generation_options_with_duration(5),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("stale boundary frames should be deleted in the same sync batch");

    let frame_prompts = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();
    assert_eq!(frame_prompts.len(), 2);
    assert!(
        frame_prompts.iter().all(|row| row.duration_secs == 5),
        "remaining frame prompts must use the regenerated duration"
    );
    assert!(
        !frame_prompts.iter().any(|row| row.shot_number == 3),
        "stale third boundary frame must be removed"
    );
}

#[tokio::test]
async fn frame_prompt_regeneration_rejects_generated_frame_images_before_llm() {
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
    insert_boundary_frame_images(db.as_ref(), project.id, chapter.id).await;

    let err = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                regenerate_generation_options_with_duration(5),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("generated frame images must block prompt regeneration before LLM");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片")
                && (message.contains("不能重新生成边界帧提示词")
                    || message.contains("不能删除边界帧提示词"))
    ));
}

#[tokio::test]
async fn frame_prompt_generation_replans_when_existing_frames_are_replaceable() {
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

    let mut changed_options = frame_generation_options();
    changed_options.duration_secs = 5;
    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned({
                    let mut replies = Vec::new();
                    replies.extend([
                        frame_prompt_json_with_duration(1, 5),
                        frame_prompt_json_with_duration(2, 5),
                        frame_prompt_json_with_duration(3, 5),
                        frame_prompt_json_with_duration(4, 5),
                    ]);
                    replies
                }),
                &engine,
                changed_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect("replaceable downstream frame prompts should not block segment planning");

    let frame_prompts = rows
        .iter()
        .filter(|row| row.prompt_type == "frame_prompt")
        .collect::<Vec<_>>();
    assert_eq!(frame_prompts.len(), 4);
    assert!(
        frame_prompts.iter().all(|row| row.duration_secs == 4),
        "reused segment planning should keep the stored segment duration"
    );
}

#[tokio::test]
async fn frame_prompt_generation_rejects_locked_incomplete_frames_before_llm() {
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

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片")
                && (message.contains("不能重新生成边界帧提示词")
                    || message.contains("不能删除边界帧提示词"))
    ));
}

#[tokio::test]
async fn frame_prompt_generation_reuses_complete_boundary_frames_without_llm_call() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let first_llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
    let mut progress = |current: u32, total: u32, message: &str| -> Result<(), AppError> {
        progress_events.push((current, total, message.to_string()));
        Ok(())
    };
    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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
        4
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
            .any(|(_, _, message)| message.contains("已复用 4 条边界帧提示词"))
    );
}

#[tokio::test]
async fn frame_prompt_resume_reuses_segment_plan_without_source_hash() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let first_llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
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

    let segment_before = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should exist after first generation");
    assert!(
        segment_before
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .and_then(|source| source.get("duration_secs_options"))
            .is_none(),
        "source metadata must not store duration candidate options"
    );
    let mut legacy_metadata = segment_before.metadata_json.clone().unwrap();
    if let Some(source) = legacy_metadata
        .get_mut("source")
        .and_then(serde_json::Value::as_object_mut)
    {
        source.remove("content_sha256");
    }
    let mut active: chapter_video_prompt::ActiveModel = segment_before.clone().into();
    active.metadata_json = Set(Some(legacy_metadata));
    let segment_before = active.update(db.as_ref()).await.unwrap();

    let resume_llm = SequenceLlmProvider::new(Vec::new());
    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &resume_llm,
                &engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .expect("segment plan without source hash should be reusable as an intermediate reference");
    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "frame_prompt")
            .count(),
        4
    );
    let segment_after = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should still exist");
    assert_eq!(segment_after.id, segment_before.id);
    assert_eq!(segment_after.metadata_json, segment_before.metadata_json);
}

#[tokio::test]
async fn frame_prompt_resume_preserves_segment_plan_and_applies_current_style_to_frames() {
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

    let segment_before = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should exist after first generation");

    let mut changed_options = frame_generation_options();
    changed_options.style = "完全不同的测试风格".to_string();
    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(boundary_frame_prompt_replies()),
                &engine,
                changed_options,
                None,
            ),
            None,
            None,
        )
        .await
        .expect("style changes should retarget the reusable segment plan instead of blocking");

    assert_eq!(
        rows.iter()
            .filter(|row| row.prompt_type == "frame_prompt")
            .count(),
        4
    );
    let segment_after = chapter_video_prompt::Entity::find_by_id(segment_before.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("reused segment plan should not be deleted");
    assert_eq!(segment_after.style, segment_before.style);
    assert_eq!(segment_after.metadata_json, segment_before.metadata_json);
    assert!(
        rows.iter()
            .filter(|row| row.prompt_type == "frame_prompt")
            .all(|row| row.style.as_deref() == Some("完全不同的测试风格"))
    );
}
