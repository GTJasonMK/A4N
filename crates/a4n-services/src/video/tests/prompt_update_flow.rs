use super::*;

#[tokio::test]
async fn update_prompt_rejects_blank_visual_prompt_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = VideoService::new(db.clone());

    let err = service
        .update_prompt(prompt.id, Some(" \n\t "), None, None, None, None)
        .await
        .expect_err("blank visual_prompt must not be saved");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "视频提示词 visual_prompt 不能为空"
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, prompt.visual_prompt);
    assert_eq!(stored.generated_video_id, None);
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn update_prompt_trims_visual_prompt_and_marks_unlinked_video_prompt_pending() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = VideoService::new(db.clone());

    let updated = service
        .update_prompt(prompt.id, Some("  新镜头提示词  "), None, None, None, None)
        .await
        .expect("valid visual_prompt should be saved after trimming");

    assert_eq!(updated.visual_prompt, "新镜头提示词");
    assert_eq!(updated.motion_prompt, Some("慢推镜头".to_string()));
    assert_eq!(updated.generated_video_id, None);
    assert_eq!(updated.status, "pending");
}

#[tokio::test]
async fn update_prompt_normalizes_blank_optional_prompts_to_none() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = VideoService::new(db.clone());

    let updated = service
        .update_prompt(
            prompt.id,
            None,
            Some(Some(" \n\t ")),
            Some(Some(" \t ")),
            None,
            None,
        )
        .await
        .expect("blank optional prompt fields should clear to null");

    assert_eq!(updated.motion_prompt, None);
    assert_eq!(updated.negative_prompt, None);
    assert_eq!(updated.generated_video_id, None);
    assert_eq!(updated.status, "pending");
}

#[tokio::test]
async fn update_prompt_rejects_generated_video_linked_prompt() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, Some(777)).await;
    let service = VideoService::new(db.clone());

    let err = service
        .update_prompt(prompt.id, Some("新镜头提示词"), None, None, None, None)
        .await
        .expect_err("generated video must lock its video prompt");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("已存在由该视频提示词生成的视频")
                && message.contains("请先删除对应视频")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, prompt.visual_prompt);
    assert_eq!(stored.generated_video_id, Some(777));
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn update_prompt_rejects_invalid_reference_image_ids_json_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = VideoService::new(db.clone());

    let err = service
        .update_prompt(
            prompt.id,
            None,
            None,
            None,
            None,
            Some(Some(json!("not-an-array"))),
        )
        .await
        .expect_err("reference_image_ids_json must be an integer array");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "reference_image_ids 必须是数组"
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.reference_image_ids_json, None);
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn update_prompt_rejects_reference_image_from_another_project_without_update() {
    let db = Arc::new(setup_db().await);
    let first_project = insert_video_project(db.as_ref(), "项目一").await;
    let second_project = insert_video_project(db.as_ref(), "项目二").await;
    let chapter = insert_video_chapter(db.as_ref(), first_project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), first_project.id, chapter.id, None).await;
    let image = insert_generated_image(db.as_ref(), second_project.id).await;
    let service = VideoService::new(db.clone());

    let err = service
        .update_prompt(
            prompt.id,
            None,
            None,
            None,
            None,
            Some(Some(json!([image.id]))),
        )
        .await
        .expect_err("cross-project reference image must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!(
                "参考图片 #{} 不属于视频项目 {} 或其根文本项目",
                image.id, first_project.id
            )
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.reference_image_ids_json, None);
}

#[tokio::test]
async fn update_prompt_rejects_reference_image_from_another_chapter_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let prompt_chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let other_chapter = insert_video_chapter(db.as_ref(), project.id, 2).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, prompt_chapter.id, None).await;
    let image =
        insert_generated_image_with_binding(db.as_ref(), project.id, Some(other_chapter.id), None)
            .await;
    let service = VideoService::new(db.clone());

    let err = service
        .update_prompt(
            prompt.id,
            None,
            None,
            None,
            None,
            Some(Some(json!([image.id]))),
        )
        .await
        .expect_err("cross-chapter reference image must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!(
                "参考图片 #{} 不属于提示词章节 {}",
                image.id, prompt_chapter.id
            )
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.reference_image_ids_json, None);
}

#[tokio::test]
async fn update_prompt_accepts_same_chapter_reference_image_with_stale_source_hash() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt_with_metadata(
        db.as_ref(),
        project.id,
        chapter.id,
        None,
        Some(json!({"source": {"content_sha256": "current"}})),
    )
    .await;
    let image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({"source": {"content_sha256": "old"}})),
    )
    .await;
    let service = VideoService::new(db.clone());

    let updated = service
        .update_prompt(
            prompt.id,
            None,
            None,
            None,
            None,
            Some(Some(json!([image.id]))),
        )
        .await
        .expect(
            "same-chapter reference image should be accepted even when its source hash is stale",
        );

    assert_eq!(updated.reference_image_ids_json, Some(json!([image.id])));
}

#[tokio::test]
async fn update_prompt_accepts_same_chapter_video_frame_reference_with_stale_source_hash() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt_with_metadata(
        db.as_ref(),
        project.id,
        chapter.id,
        None,
        Some(json!({"source": {"content_sha256": "current"}})),
    )
    .await;
    let image = insert_generated_image_with_binding(
        db.as_ref(),
        project.id,
        Some(chapter.id),
        Some(json!({
            "source": {"content_sha256": "old"},
            "video_frame_role": "key_frame",
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": 101,
                "chapter_id": chapter.id,
                "project_id": project.id,
                "shot_number": 1
            }
        })),
    )
    .await;
    let service = VideoService::new(db.clone());

    let updated = service
        .update_prompt(
            prompt.id,
            None,
            None,
            None,
            None,
            Some(Some(json!([image.id]))),
        )
        .await
        .expect("same-chapter video frame image should be reusable as a frame reference");

    assert_eq!(updated.reference_image_ids_json, Some(json!([image.id])));
}

#[tokio::test]
async fn update_prompt_accepts_project_reference_image_and_marks_unlinked_video_prompt_pending() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let image = insert_generated_image(db.as_ref(), project.id).await;
    let service = VideoService::new(db.clone());

    let updated = service
        .update_prompt(
            prompt.id,
            None,
            None,
            None,
            None,
            Some(Some(json!([image.id]))),
        )
        .await
        .expect("project-scoped reference image should be valid");

    assert_eq!(updated.reference_image_ids_json, Some(json!([image.id])));
    assert_eq!(updated.generated_video_id, None);
    assert_eq!(updated.status, "pending");
}

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
    let service = VideoService::new(db.clone());

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
    let service = VideoService::new(db.clone());

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
        "first_frame_segment_plan",
        "first_frame_chain",
        0,
        json!({
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
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
    let service = VideoService::new(db.clone());

    let err = service
        .delete_prompt(segment.id)
        .await
        .expect_err("first-frame prompts must lock earlier segment plan");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("后续首帧提示词")
                && message.contains("不能删除首帧串联片段规划")
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
    let service = VideoService::new(db.clone());

    let err = service
        .delete_prompt(first_frame.id)
        .await
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
    let service = VideoService::new(db.clone());

    service
        .ensure_frame_prompt_image_generation_allowed(frame.id, "video_boundary_frame_prompt")
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
                "source": "video_boundary_frame_prompt",
                "prompt_id": frame.id
            }
        })),
    )
    .await;
    let service = VideoService::new(db.clone());

    let err = service
        .ensure_frame_prompt_image_generation_allowed(frame.id, "video_boundary_frame_prompt")
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
    let service = VideoService::new(db.clone());

    let err = service
        .ensure_frame_prompt_image_generation_allowed(first_frame.id, "video_boundary_frame_prompt")
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
                "source": "video_boundary_frame_prompt",
                "prompt_id": first_frame.id
            }
        })),
    )
    .await;
    let material_image = insert_generated_image(db.as_ref(), project.id).await;
    let service = VideoService::new(db.clone());

    let ids = service
        .effective_frame_prompt_image_reference_ids(
            second_frame.id,
            "video_boundary_frame_prompt",
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
    let service = VideoService::new(db.clone());

    let err = service
        .effective_frame_prompt_image_reference_ids(
            second_frame.id,
            "video_boundary_frame_prompt",
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

#[tokio::test]
async fn sync_prompts_rejects_regenerating_frame_prompt_with_generated_image() {
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
    insert_generated_image_with_binding(
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
    let service = VideoService::new(db.clone());

    let err = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "frame_prompt",
                shot_number: 1,
                title: Some("重生成边界帧"),
                scene_summary: Some("重生成摘要"),
                visual_prompt: "这是重新生成后的边界帧提示词，内容已经变化",
                motion_prompt: None,
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(json!({
                    "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same"}
                })),
            })],
        )
        .await
        .expect_err("prompt sync must not rewrite frame prompt after image generation");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("边界帧图片") && message.contains("不能重新生成边界帧提示词")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(frame.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, frame.visual_prompt);
}

async fn insert_stage_prompt(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    shot_number: i32,
    metadata_json: Value,
) -> chapter_video_prompt::Model {
    let now = fixed_time(4);
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set(prompt_type.to_string()),
        workflow_mode: Set(workflow_mode.to_string()),
        shot_number: Set(shot_number),
        title: Set(Some("阶段提示词".to_string())),
        scene_summary: Set(Some("阶段摘要".to_string())),
        visual_prompt: Set("阶段视觉提示词".to_string()),
        motion_prompt: Set(Some("阶段动作提示词".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(metadata_json)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}
