#[tokio::test]
async fn update_prompt_rejects_blank_visual_prompt_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                visual_prompt: Some(" \n\t ".to_string()),
                ..Default::default()
            },
            false,
        )
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let updated = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                visual_prompt: Some("  新镜头提示词  ".to_string()),
                ..Default::default()
            },
            false,
        )
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let updated = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                motion_prompt: Some(Some(" \n\t ".to_string())),
                negative_prompt: Some(Some(" \t ".to_string())),
                ..Default::default()
            },
            false,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                visual_prompt: Some("新镜头提示词".to_string()),
                ..Default::default()
            },
            false,
        )
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                reference_image_ids_json: Some(Some(json!("not-an-array"))),
                ..Default::default()
            },
            false,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                reference_image_ids_json: Some(Some(json!([image.id]))),
                ..Default::default()
            },
            false,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                reference_image_ids_json: Some(Some(json!([image.id]))),
                ..Default::default()
            },
            false,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let updated = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                reference_image_ids_json: Some(Some(json!([image.id]))),
                ..Default::default()
            },
            false,
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
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
                "prompt_id": 101,
                "chapter_id": chapter.id,
                "project_id": project.id,
                "shot_number": 1
            }
        })),
    )
    .await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let updated = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                reference_image_ids_json: Some(Some(json!([image.id]))),
                ..Default::default()
            },
            false,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let updated = service
        .update_chapter_video_prompt_with_policy(
            prompt.id,
            ChapterVideoPromptPatch {
                reference_image_ids_json: Some(Some(json!([image.id]))),
                ..Default::default()
            },
            false,
        )
        .await
        .expect("project-scoped reference image should be valid");

    assert_eq!(updated.reference_image_ids_json, Some(json!([image.id])));
    assert_eq!(updated.generated_video_id, None);
    assert_eq!(updated.status, "pending");
}
