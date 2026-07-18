#[tokio::test]
async fn sync_chapter_prompts_rejects_missing_chapter_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .sync_chapter_prompts(
            404,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: 404,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("镜头一"),
                scene_summary: Some("雨夜对峙"),
                visual_prompt: "雨夜街巷，人物对峙",
                motion_prompt: Some("慢推镜头"),
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(json!({
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same"}
                })),
            })],
        )
        .await
        .expect_err("missing chapter must not create orphan video prompts");

    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "章节不存在: 404"
    ));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn sync_chapter_prompts_keeps_generated_video_prompt_when_language_changes() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let zh_rows = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("镜头一"),
                scene_summary: Some("雨夜对峙"),
                visual_prompt: "中文视频提示词",
                motion_prompt: Some("中文动作提示词"),
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(video_prompt_metadata_with_language("zh_cn")),
            })],
        )
        .await
        .expect("zh prompt should be inserted");
    let mut active: chapter_video_prompt::ActiveModel = zh_rows[0].clone().into();
    active.generated_video_id = Set(Some(9301));
    active.status = Set("completed".to_string());
    active.update(db.as_ref()).await.unwrap();

    let en_rows = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("Shot One"),
                scene_summary: Some("Rainy standoff"),
                visual_prompt: "English video prompt",
                motion_prompt: Some("English motion prompt"),
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(video_prompt_metadata_with_language("en")),
            })],
        )
        .await
        .expect("en prompt should be inserted without overwriting zh generated-video prompt");

    assert_eq!(en_rows.len(), 1);
    assert_ne!(en_rows[0].id, zh_rows[0].id);
    assert_eq!(en_rows[0].generated_video_id, None);

    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().any(|row| {
        row.generated_video_id == Some(9301)
            && row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("prompt_output_language"))
                .and_then(serde_json::Value::as_str)
                == Some("zh_cn")
    }));
    assert!(stored.iter().any(|row| {
        row.generated_video_id.is_none()
            && row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("prompt_output_language"))
                .and_then(serde_json::Value::as_str)
                == Some("en")
    }));
}

#[tokio::test]
async fn sync_chapter_prompts_keeps_frame_prompt_when_language_changes() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let zh_rows = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some("电影感写实"),
                prompt_type: "first_frame_prompt",
                shot_number: 1,
                title: Some("首帧"),
                scene_summary: Some("中文首帧摘要"),
                visual_prompt: "中文首帧提示词",
                motion_prompt: None,
                negative_prompt: Some("低清晰度"),
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(first_frame_prompt_metadata_with_language("zh_cn")),
            })],
        )
        .await
        .expect("zh first-frame prompt should be inserted");

    let en_rows = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some("电影感写实"),
                prompt_type: "first_frame_prompt",
                shot_number: 1,
                title: Some("First Frame"),
                scene_summary: Some("English first-frame summary"),
                visual_prompt: "English first-frame prompt",
                motion_prompt: None,
                negative_prompt: Some("low quality"),
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(first_frame_prompt_metadata_with_language("en")),
            })],
        )
        .await
        .expect("en first-frame prompt should not delete zh first-frame prompt");

    assert_eq!(en_rows.len(), 1);
    assert_ne!(en_rows[0].id, zh_rows[0].id);

    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .filter(chapter_video_prompt::Column::PromptType.eq("first_frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().any(|row| {
        row.visual_prompt == "中文首帧提示词"
            && row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("prompt_output_language"))
                .and_then(serde_json::Value::as_str)
                == Some("zh_cn")
    }));
    assert!(stored.iter().any(|row| {
        row.visual_prompt == "English first-frame prompt"
            && row
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("prompt_output_language"))
                .and_then(serde_json::Value::as_str)
                == Some("en")
    }));
}

#[tokio::test]
async fn sync_chapter_prompts_rejects_draft_for_another_chapter_without_insert() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let first_project = insert_video_project(db.as_ref(), "项目一").await;
    let second_project = insert_video_project(db.as_ref(), "项目二").await;
    let first_chapter = insert_video_chapter(db.as_ref(), first_project.id, 1).await;
    let second_chapter = insert_video_chapter(db.as_ref(), second_project.id, 1).await;

    let err = service
        .sync_chapter_prompts(
            first_chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: first_project.id,
                chapter_id: second_chapter.id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("镜头一"),
                scene_summary: Some("雨夜对峙"),
                visual_prompt: "雨夜街巷，人物对峙",
                motion_prompt: Some("慢推镜头"),
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(json!({
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same"}
                })),
            })],
        )
        .await
        .expect_err("draft for another chapter must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!(
                "视频提示词草稿章节 {} 与同步章节 {} 不一致",
                second_chapter.id, first_chapter.id
            )
    ));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn sync_chapter_prompts_rejects_draft_project_mismatch_without_insert() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let project = insert_video_project(db.as_ref(), "项目一").await;
    let other_project = insert_video_project(db.as_ref(), "项目二").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let err = service
        .sync_chapter_prompts(
            chapter.id,
            vec![
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id: project.id,
                    chapter_id: chapter.id,
                    workflow_mode: PromptWorkflowMode::BoundaryFrames,
                    style: Some("电影感写实"),
                    prompt_type: "frame_segment_plan",
                    shot_number: 0,
                    title: Some("片段规划"),
                    scene_summary: Some("雨夜对峙"),
                    visual_prompt: "雨夜街巷，人物对峙",
                    motion_prompt: None,
                    negative_prompt: None,
                    duration_secs: 6,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: None,
                    metadata_json: Some(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"}
                    })),
                }),
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id: other_project.id,
                    chapter_id: chapter.id,
                    workflow_mode: PromptWorkflowMode::BoundaryFrames,
                    style: Some("电影感写实"),
                    prompt_type: "video_prompt",
                    shot_number: 1,
                    title: Some("镜头一"),
                    scene_summary: Some("雨夜对峙"),
                    visual_prompt: "雨夜街巷，人物对峙",
                    motion_prompt: Some("慢推镜头"),
                    negative_prompt: None,
                    duration_secs: 6,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: None,
                    metadata_json: Some(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"}
                    })),
                }),
            ],
        )
        .await
        .expect_err("draft for another project must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!(
                "视频提示词草稿项目 {} 与章节所属项目 {} 不一致",
                other_project.id, project.id
            )
    ));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn sync_chapter_prompts_rejects_unknown_reference_image_without_insert() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;

    let err = service
        .sync_chapter_prompts(
            chapter.id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("镜头一"),
                scene_summary: Some("雨夜对峙"),
                visual_prompt: "雨夜街巷，人物对峙",
                motion_prompt: Some("慢推镜头"),
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([404])),
                metadata_json: Some(json!({
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same"}
                })),
            })],
        )
        .await
        .expect_err("unknown reference image must not be persisted");

    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "参考图片不存在: 404"
    ));
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn missing_chapter_prompt_queries_return_not_found_without_touching_orphans() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, 999, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .ensure_chapter_belongs_to_project(project.id, 999)
        .await
        .expect_err("missing chapter list must be visible");
    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "章节不存在: 999"
    ));

    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.chapter_id, 999);
    assert_eq!(stored.status, "completed");
}
