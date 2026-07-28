#[tokio::test]
async fn update_prompt_rejects_frame_prompt_when_video_prompts_exist() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "video_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_prompt(frame.id, Some("新的边界帧提示词"), None, None, None, None)
        .await
        .expect_err("video prompt must lock earlier frame prompt");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("后续视频提示词") && message.contains("不能编辑边界帧提示词")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(frame.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, frame.visual_prompt);
}

#[tokio::test]
async fn update_prompt_rejects_frame_prompt_when_generated_frame_image_exists() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    let image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": frame.id
            }
        })),
    )
    .await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_prompt(frame.id, Some("新的边界帧提示词"), None, None, None, None)
        .await
        .expect_err("generated frame image must lock its frame prompt");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片")
                && message.contains(&format!("#{}", image.id))
                && message.contains("不能编辑边界帧提示词")
                && message.contains("请先删除对应图片")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(frame.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, frame.visual_prompt);
}

#[tokio::test]
async fn delete_prompt_rejects_segment_plan_when_frame_prompts_exist() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let segment = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_segment_plan",
        "shared_segment_plan",
        0,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "first_frame_prompt",
        "first_frame_chain",
        1,
        json!({
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    let err = a4n_db::video_prompt::delete_chapter_video_prompt_with_policy(
        db.as_ref(),
        segment.id,
        false,
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect_err("first-frame prompts must lock earlier segment plan");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("后续首帧提示词")
                && message.contains("不能删除视频片段规划")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(segment.id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(stored.is_some());
}

#[tokio::test]
async fn delete_prompt_rejects_first_frame_prompt_when_generated_first_frame_image_exists() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let first_frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "first_frame_prompt",
        "first_frame_chain",
        1,
        json!({
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }),
    )
    .await;
    let image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": "video_first_frame_prompt",
                "prompt_id": first_frame.id
            }
        })),
    )
    .await;
    let err = a4n_db::video_prompt::delete_chapter_video_prompt_with_policy(
        db.as_ref(),
        first_frame.id,
        false,
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect_err("generated first-frame image must lock its prompt");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("首帧图片")
                && message.contains(&format!("#{}", image.id))
                && message.contains("不能删除首帧提示词")
                && message.contains("请先删除对应图片")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(first_frame.id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(stored.is_some());
}
