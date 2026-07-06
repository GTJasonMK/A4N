use super::*;

#[tokio::test]
async fn video_prompt_generation_writes_segment_aligned_video_prompts_after_boundary_frames() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_llm = SequenceLlmProvider::new_owned(boundary_frame_generation_replies());

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

    let stored_frame_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_frame_count, 4);
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
        3
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
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(3))
        .exec(db.as_ref())
        .await
        .unwrap();

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
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

#[tokio::test]
async fn boundary_frame_generation_resumes_after_deleted_middle_frame_without_replanning() {
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
    let segment_plan = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should exist");
    let first_frame = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first frame prompt should exist");
    let first_frame_image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": first_frame.id
            },
            "source": video_source_metadata(&generation_source())
        })),
    )
    .await;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(2))
        .exec(db.as_ref())
        .await
        .unwrap();

    let rows = service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![frame_prompt_json_with_references(
                    2,
                    &[first_frame_image.id],
                )]),
                &engine,
                video_generation_options_with_batch_size(1),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(rows.iter().any(|row| {
        row.prompt_type == "frame_prompt"
            && row.shot_number == 2
            && row.reference_image_ids_json == Some(json!([first_frame_image.id]))
    }));
    let current_segment_plan_id = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("segment plan should still exist")
        .id;
    assert_eq!(current_segment_plan_id, segment_plan.id);
    let frame_numbers = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.shot_number)
        .collect::<Vec<_>>();
    assert_eq!(frame_numbers, vec![1, 2]);
}

#[tokio::test]
async fn boundary_frame_partial_generation_preserves_later_prompt_with_generated_image() {
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
    let third_frame = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(3))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("third frame prompt should exist");
    let third_frame_image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": third_frame.id
            },
            "source": video_source_metadata(&generation_source())
        })),
    )
    .await;
    let reference_image = insert_generated_image(db.as_ref(), project.id).await;
    let mut options = video_generation_options_with_batch_size(1);
    options.reference_image_ids = vec![reference_image.id];
    options.video_library_image_ids = vec![reference_image.id];

    service
        .generate_chapter_boundary_frame_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![frame_prompt_json(1)]),
                &engine,
                options,
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();

    let preserved_third_frame = chapter_video_prompt::Entity::find_by_id(third_frame.id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(
        preserved_third_frame.is_some(),
        "partial boundary frame checkpoint must preserve later frame prompts that already produced images"
    );
    let preserved_image = generated_image::Entity::find_by_id(third_frame_image.id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(preserved_image.is_some());
}

#[tokio::test]
async fn video_prompt_generation_reports_missing_boundary_frame_when_partial_chain_is_exhausted() {
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
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(3))
        .exec(db.as_ref())
        .await
        .unwrap();

    service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new_owned(vec![video_shot_json(1), video_shot_json(1)]),
                &engine,
                video_generation_options_with_batch_size(1),
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
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first video prompt should exist");
    let mut active: chapter_video_prompt::ActiveModel = first_prompt.into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let err = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
                &SequenceLlmProvider::new(Vec::new()),
                &engine,
                video_generation_options_with_batch_size(2),
                None,
            ),
            None,
            None,
        )
        .await
        .expect_err("exhausted partial boundary chain must explain the missing frame");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("缺少边界帧提示词 #3")
                && message.contains("下一条视频提示词 #2")
    ));
}

#[tokio::test]
async fn boundary_video_prompt_generation_respects_batch_size_and_preserves_existing_prompts() {
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

    let batch_one_options = video_generation_options_with_batch_size(1);
    let first_rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
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
    let first_video_rows = first_rows
        .iter()
        .filter(|row| row.prompt_type == "video_prompt")
        .collect::<Vec<_>>();
    assert_eq!(first_video_rows.len(), 1);
    assert_eq!(first_video_rows[0].shot_number, 1);

    let rows = service
        .generate_chapter_video_prompts(
            prompt_stage_input(
                project.id,
                chapter.id,
                "boundary_frames",
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
    let stored_video_count = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("boundary_frames"))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored_video_count, 2);
}
