#[tokio::test]
async fn frame_image_generation_allows_prompt_with_later_video_prompt() {
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

    service
        .ensure_frame_prompt_image_generation_allowed(
            frame.id,
            IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        )
        .await
        .expect("later video prompts must not block frame image generation");
}

#[tokio::test]
async fn frame_image_generation_rejects_prompt_with_generated_frame_image() {
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
                "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
                "prompt_id": frame.id
            }
        })),
    )
    .await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .ensure_frame_prompt_image_generation_allowed(
            frame.id,
            IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        )
        .await
        .expect_err("generated frame image must block duplicate frame image generation");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片")
                && message.contains(&format!("#{}", image.id))
                && message.contains("不能成图边界帧提示词")
    ));
}

#[tokio::test]
async fn frame_image_generation_rejects_mismatched_generation_source() {
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .ensure_frame_prompt_image_generation_allowed(
            first_frame.id,
            IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        )
        .await
        .expect_err("source must match prompt type");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("视频帧成图来源 video_boundary_frame_prompt 与首帧提示词不匹配")
    ));
}

#[tokio::test]
async fn boundary_frame_generation_references_previous_frame_image_first() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let metadata = json!({
        "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        "source_prompt_type": "frame_segment_plan",
        "source": {
            "content_sha256": "same"
        },
        "generation_asset_fingerprint": {
            "schema_version": 1,
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        }
    });
    let first_frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        1,
        metadata.clone(),
    )
    .await;
    let second_frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        2,
        metadata,
    )
    .await;
    let previous_image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
                "prompt_id": first_frame.id
            }
        })),
    )
    .await;
    let material_image = insert_generated_image(db.as_ref(), project.id).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let ids = service
        .effective_frame_prompt_image_reference_ids(
            second_frame.id,
            IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
            &[material_image.id],
        )
        .await
        .expect("previous boundary frame image should be injected first");

    assert_eq!(ids, vec![previous_image.id, material_image.id]);
}

#[tokio::test]
async fn boundary_frame_generation_rejects_missing_previous_frame_image() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let metadata = json!({
        "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        "source_prompt_type": "frame_segment_plan",
        "source": {
            "content_sha256": "same"
        },
        "generation_asset_fingerprint": {
            "schema_version": 1,
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        }
    });
    insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        1,
        metadata.clone(),
    )
    .await;
    let second_frame = insert_stage_prompt(
        db.as_ref(),
        project.id,
        chapter.id,
        "frame_prompt",
        "boundary_frames",
        2,
        metadata,
    )
    .await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .effective_frame_prompt_image_reference_ids(
            second_frame.id,
            IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
            &[],
        )
        .await
        .expect_err("missing previous frame image must block boundary frame generation");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("先生成第 1 张边界帧图片")
                && message.contains("参考图 1")
    ));
}
