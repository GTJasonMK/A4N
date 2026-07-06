use super::*;

fn test_generation_asset_fingerprint() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "reference_image_ids": [],
        "video_library_image_ids": [],
        "available_assets_sha256": "test-assets"
    })
}

fn video_prompt_metadata_with_language(language: &str) -> serde_json::Value {
    json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same"},
        "contract_version": "boundary_frame_incremental_v1",
        "prompt_output_language": language,
        "generation_asset_fingerprint": test_generation_asset_fingerprint()
    })
}

fn first_frame_prompt_metadata_with_language(language: &str) -> serde_json::Value {
    json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same"},
        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
        "prompt_output_language": language,
        "generation_asset_fingerprint": test_generation_asset_fingerprint()
    })
}

#[tokio::test]
async fn sync_chapter_prompts_rejects_missing_chapter_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = VideoService::new(db.clone());

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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());

    let err = service
        .list_prompts_by_chapter(project.id, 999)
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

#[tokio::test]
async fn sync_chapter_prompts_preserves_unchanged_video_prompt_identity() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let ref_image_a = insert_generated_image(db.as_ref(), project_id).await;
    let ref_image_b = insert_generated_image(db.as_ref(), project_id).await;
    let reference_image_ids = json!([ref_image_a.id, ref_image_b.id]);
    let old_time = fixed_time(1);
    let kept = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(1),
        title: Set(Some("镜头一".to_string())),
        scene_summary: Set(Some("雨夜对峙".to_string())),
        visual_prompt: Set("雨夜街巷，人物对峙".to_string()),
        motion_prompt: Set(Some("慢推镜头".to_string())),
        negative_prompt: Set(Some("低清晰度".to_string())),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(reference_image_ids.clone())),
        status: Set("completed".to_string()),
        generated_video_id: Set(Some(9301)),
        metadata_json: Set(Some(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let stale = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(9),
        visual_prompt: Set("旧镜头".to_string()),
        duration_secs: Set(4),
        status: Set("pending".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .sync_chapter_prompts(
            chapter_id,
            vec![
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::BoundaryFrames,
                    style: Some("电影感写实"),
                    prompt_type: "video_prompt",
                    shot_number: 1,
                    title: Some("镜头一"),
                    scene_summary: Some("雨夜对峙"),
                    visual_prompt: "雨夜街巷，人物对峙",
                    motion_prompt: Some("慢推镜头"),
                    negative_prompt: Some("低清晰度"),
                    duration_secs: 6,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: Some(reference_image_ids.clone()),
                    metadata_json: Some(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"}
                    })),
                }),
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::BoundaryFrames,
                    style: Some("电影感写实"),
                    prompt_type: "video_prompt",
                    shot_number: 2,
                    title: Some("镜头二"),
                    scene_summary: Some("追逐"),
                    visual_prompt: "巷口追逐",
                    motion_prompt: Some("跟拍"),
                    negative_prompt: None,
                    duration_secs: 5,
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
        .unwrap();

    assert_eq!(rows.len(), 2);
    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].generated_video_id, Some(9301));
    assert_eq!(stored[0].status, "completed");
    assert_eq!(stored[0].updated_at, old_time);
    assert!(!stored.iter().any(|row| row.id == stale.id));
}

#[tokio::test]
async fn sync_chapter_prompts_preserving_generated_video_prompts_keeps_changed_bound_prompt() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let old_time = fixed_time(1);
    let kept = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("first_frame_chain".to_string()),
        shot_number: Set(1),
        title: Set(Some("旧镜头".to_string())),
        scene_summary: Set(Some("旧摘要".to_string())),
        visual_prompt: Set("旧视觉提示词".to_string()),
        motion_prompt: Set(Some("旧动作提示词".to_string())),
        negative_prompt: Set(Some("旧负面提示词".to_string())),
        duration_secs: Set(4),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!([]))),
        status: Set("completed".to_string()),
        generated_video_id: Set(Some(9301)),
        metadata_json: Set(Some(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "prompt_output_language": "zh_cn",
            "generation_asset_fingerprint": test_generation_asset_fingerprint()
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .sync_chapter_prompts_preserving_generated_video_prompts(
            chapter_id,
            vec![
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::FirstFrameChain,
                    style: Some("电影感写实"),
                    prompt_type: "video_prompt",
                    shot_number: 1,
                    title: Some("新镜头"),
                    scene_summary: Some("新摘要"),
                    visual_prompt: "新视觉提示词",
                    motion_prompt: Some("新动作提示词"),
                    negative_prompt: Some("新负面提示词"),
                    duration_secs: 4,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: Some(json!([])),
                    metadata_json: Some(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"},
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "prompt_output_language": "zh_cn",
                        "generation_asset_fingerprint": test_generation_asset_fingerprint()
                    })),
                }),
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::FirstFrameChain,
                    style: Some("电影感写实"),
                    prompt_type: "video_prompt",
                    shot_number: 2,
                    title: Some("后续镜头"),
                    scene_summary: Some("后续摘要"),
                    visual_prompt: "后续视觉提示词",
                    motion_prompt: Some("后续动作提示词"),
                    negative_prompt: None,
                    duration_secs: 4,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: Some(json!([])),
                    metadata_json: Some(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"},
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "prompt_output_language": "zh_cn",
                        "generation_asset_fingerprint": test_generation_asset_fingerprint()
                    })),
                }),
            ],
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].title.as_deref(), Some("旧镜头"));
    assert_eq!(stored[0].visual_prompt, "旧视觉提示词");
    assert_eq!(stored[0].generated_video_id, Some(9301));
    assert_eq!(stored[0].updated_at, old_time);
    assert_eq!(stored[1].shot_number, 2);
    assert_eq!(stored[1].generated_video_id, None);
}

#[tokio::test]
async fn sync_chapter_prompts_preserving_generated_frame_prompts_keeps_changed_bound_prompt() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let old_time = fixed_time(2);
    let kept = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("frame_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(1),
        title: Set(Some("旧边界帧".to_string())),
        scene_summary: Set(Some("旧摘要".to_string())),
        visual_prompt: Set("旧边界帧提示词".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(Some("旧负面提示词".to_string())),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!([]))),
        status: Set("pending".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"},
            "contract_version": "boundary_frame_incremental_v1",
            "prompt_output_language": "zh_cn",
            "generation_asset_fingerprint": {
                "schema_version": 1,
                "reference_image_ids": [],
                "video_library_image_ids": [],
                "available_assets_sha256": "old-assets"
            },
            "boundary_frame": {
                "frame_number": 1,
                "title": "旧边界帧",
                "scene_summary": "旧摘要",
                "frame_prompt": "旧边界帧提示词",
                "blocking_state": {},
                "lighting_state": {},
                "negative_prompt": "旧负面提示词",
                "duration_secs": 6,
                "aspect_ratio": "16:9",
                "reference_image_ids": [],
                "segment_numbers": [1]
            }
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    insert_generated_image_with_binding(
        db.as_ref(),
        project_id,
        Some(chapter_id),
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": kept.id
            }
        })),
    )
    .await;

    let rows = service
        .sync_chapter_prompts_preserving_generated_frame_prompts(
            chapter_id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "frame_prompt",
                shot_number: 1,
                title: Some("新边界帧"),
                scene_summary: Some("新摘要"),
                visual_prompt: "新边界帧提示词",
                motion_prompt: None,
                negative_prompt: Some("新负面提示词"),
                duration_secs: 6,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(json!({
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same"},
                    "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
                    "prompt_output_language": "zh_cn",
                    "generation_asset_fingerprint": {
                        "schema_version": 1,
                        "reference_image_ids": [],
                        "video_library_image_ids": [],
                        "available_assets_sha256": "new-assets"
                    },
                    "boundary_frame": {
                        "frame_number": 1,
                        "title": "新边界帧",
                        "scene_summary": "新摘要",
                        "frame_prompt": "新边界帧提示词",
                        "blocking_state": {},
                        "lighting_state": {},
                        "negative_prompt": "新负面提示词",
                        "duration_secs": 6,
                        "aspect_ratio": "16:9",
                        "reference_image_ids": [],
                        "segment_numbers": [1]
                    }
                })),
            })],
        )
        .await
        .expect("generated frame prompt should be preserved instead of overwritten");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, kept.id);
    assert_eq!(rows[0].visual_prompt, "旧边界帧提示词");
    assert_eq!(rows[0].updated_at, old_time);

    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].visual_prompt, "旧边界帧提示词");
}

#[tokio::test]
async fn sync_chapter_prompts_scopes_deletion_to_generated_prompt_types() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let old_time = fixed_time(2);
    let video_prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(1),
        title: Set(Some("保留镜头".to_string())),
        visual_prompt: Set("已有最终视频提示词".to_string()),
        duration_secs: Set(6),
        status: Set("pending".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    service
        .sync_chapter_prompts(
            chapter_id,
            vec![
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::BoundaryFrames,
                    style: Some("电影感写实"),
                    prompt_type: "frame_segment_plan",
                    shot_number: 0,
                    title: Some("视频片段规划"),
                    scene_summary: None,
                    visual_prompt: r#"{"segments":[]}"#,
                    motion_prompt: None,
                    negative_prompt: None,
                    duration_secs: 6,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: None,
                    metadata_json: Some(json!({"source": {"content_sha256": "same"}})),
                }),
                new_prompt_draft(NewChapterVideoPromptInput {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::BoundaryFrames,
                    style: Some("电影感写实"),
                    prompt_type: "frame_prompt",
                    shot_number: 1,
                    title: Some("殿门首帧"),
                    scene_summary: None,
                    visual_prompt: "小宦官站在殿门前，烛光从画面右侧照入。",
                    motion_prompt: None,
                    negative_prompt: Some("低清晰度"),
                    duration_secs: 6,
                    aspect_ratio: Some("16:9"),
                    reference_image_ids_json: None,
                    metadata_json: Some(json!({"source": {"content_sha256": "same"}})),
                }),
            ],
        )
        .await
        .unwrap();

    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::PromptType)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db.as_ref())
        .await
        .unwrap();

    assert_eq!(stored.len(), 3);
    assert!(stored.iter().any(|row| row.id == video_prompt.id));
    assert!(
        stored
            .iter()
            .any(|row| row.prompt_type == "frame_segment_plan")
    );
    assert!(stored.iter().any(|row| row.prompt_type == "frame_prompt"));
}

#[tokio::test]
async fn sync_chapter_prompts_keeps_video_prompt_workflows_separate() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let project_id = project.id;
    let chapter_id = chapter.id;
    let old_time = fixed_time(4);

    let first_frame_prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("first_frame_chain".to_string()),
        shot_number: Set(1),
        title: Set(Some("首帧串联镜头一".to_string())),
        scene_summary: Set(Some("首帧串联旧摘要".to_string())),
        visual_prompt: Set("首帧串联旧提示词".to_string()),
        motion_prompt: Set(Some("从首帧开始运动".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        status: Set("pending".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same-source"}
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let stale_boundary_prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(9),
        title: Set(Some("边界帧旧镜头".to_string())),
        visual_prompt: Set("应该被边界帧同步删除的旧提示词".to_string()),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        status: Set("pending".to_string()),
        metadata_json: Set(Some(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same-source"}
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .sync_chapter_prompts(
            chapter_id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("边界帧镜头一"),
                scene_summary: Some("边界帧摘要"),
                visual_prompt: "边界帧模式的新提示词",
                motion_prompt: Some("从首尾边界帧插值"),
                negative_prompt: None,
                duration_secs: 8,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(json!({
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same-source"}
                })),
            })],
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_ne!(rows[0].id, first_frame_prompt.id);
    assert_eq!(rows[0].title.as_deref(), Some("边界帧镜头一"));

    let kept_first_frame_prompt = chapter_video_prompt::Entity::find_by_id(first_frame_prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("first-frame-chain prompt must not be overwritten by boundary sync");
    assert_eq!(
        kept_first_frame_prompt.title.as_deref(),
        Some("首帧串联镜头一")
    );
    assert_eq!(kept_first_frame_prompt.workflow_mode, "first_frame_chain");
    assert_eq!(kept_first_frame_prompt.visual_prompt, "首帧串联旧提示词");
    assert!(
        chapter_video_prompt::Entity::find_by_id(stale_boundary_prompt.id)
            .one(db.as_ref())
            .await
            .unwrap()
            .is_none(),
        "same-workflow stale boundary video prompt should still be deleted"
    );

    let stored = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().any(|row| row.id == first_frame_prompt.id));
    assert!(stored.iter().any(|row| row.id == rows[0].id));
}

#[tokio::test]
async fn sync_chapter_prompts_rejects_metadata_workflow_that_conflicts_with_explicit_workflow() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
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
                title: Some("冲突镜头"),
                scene_summary: Some("冲突摘要"),
                visual_prompt: "这条草稿显式属于边界帧链，但 metadata 声称首帧链",
                motion_prompt: Some("慢推"),
                negative_prompt: None,
                duration_secs: 8,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(json!({
                    "workflow_mode": "first_frame_chain",
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same-source"}
                })),
            })],
        )
        .await
        .expect_err("metadata workflow conflict must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("metadata.workflow_mode=first_frame_chain")
                && message.contains("显式 workflow_mode=boundary_frames")
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
async fn matching_prompt_rows_finds_first_frame_video_prompt_by_workflow_contract() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let source_metadata = json!({"content_sha256": "first-frame-source"});
    let asset_fingerprint = test_generation_asset_fingerprint();
    let old_time = fixed_time(5);
    let first_frame_video_prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("first_frame_chain".to_string()),
        shot_number: Set(1),
        title: Set(Some("首帧串联镜头一".to_string())),
        visual_prompt: Set("首帧串联最终视频提示词".to_string()),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        status: Set("pending".to_string()),
        metadata_json: Set(Some(json!({
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "source": source_metadata.clone()
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: "first_frame_chain",
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: "电影感写实",
                duration_secs: 8,
                aspect_ratio: "16:9",
                reference_ids: &json!([]),
                generation_asset_fingerprint: Some(&asset_fingerprint),
                source_prompt_type: Some("frame_segment_plan"),
            },
            PromptOutputLanguage::ZhCn,
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, first_frame_video_prompt.id);
    assert_eq!(rows[0].workflow_mode, "first_frame_chain");
}

#[tokio::test]
async fn matching_prompt_rows_rejects_different_duration() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let source_metadata = json!({"content_sha256": "same-duration-source"});
    let asset_fingerprint = test_generation_asset_fingerprint();
    let old_time = fixed_time(5);
    let video_prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("chat_video".to_string()),
        shot_number: Set(1),
        title: Set(Some("8 秒旧提示词".to_string())),
        visual_prompt: Set("这是 8 秒旧 Chat 视频提示词".to_string()),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        status: Set("pending".to_string()),
        metadata_json: Set(Some(json!({
            "contract_version": CHAT_VIDEO_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_reference_image_ids": [],
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "source": source_metadata.clone()
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let five_second_rows = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: "chat_video",
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: "电影感写实",
                duration_secs: 5,
                aspect_ratio: "16:9",
                reference_ids: &json!([]),
                generation_asset_fingerprint: Some(&asset_fingerprint),
                source_prompt_type: Some("frame_segment_plan"),
            },
            PromptOutputLanguage::ZhCn,
        )
        .await
        .unwrap();
    assert!(
        five_second_rows.is_empty(),
        "5 秒生成不能复用数据库里的 8 秒旧提示词"
    );

    let eight_second_rows = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: "chat_video",
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: "电影感写实",
                duration_secs: 8,
                aspect_ratio: "16:9",
                reference_ids: &json!([]),
                generation_asset_fingerprint: Some(&asset_fingerprint),
                source_prompt_type: Some("frame_segment_plan"),
            },
            PromptOutputLanguage::ZhCn,
        )
        .await
        .unwrap();
    assert_eq!(eight_second_rows.len(), 1);
    assert_eq!(eight_second_rows[0].id, video_prompt.id);
}

#[tokio::test]
async fn matching_prompt_rows_is_scoped_to_project() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let first_project = insert_video_project(db.as_ref(), "视频项目一").await;
    let second_project = insert_video_project(db.as_ref(), "视频项目二").await;
    let first_chapter = insert_video_chapter(db.as_ref(), first_project.id, 1).await;
    let source_metadata = json!({"content_sha256": "same-root-chapter"});
    let asset_fingerprint = test_generation_asset_fingerprint();
    let old_time = fixed_time(6);
    chapter_video_prompt::ActiveModel {
        project_id: Set(first_project.id),
        chapter_id: Set(first_chapter.id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("first_frame_chain".to_string()),
        shot_number: Set(1),
        title: Set(Some("另一个派生项目的提示词".to_string())),
        visual_prompt: Set("这条记录不能被当前项目复用".to_string()),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        status: Set("pending".to_string()),
        metadata_json: Set(Some(json!({
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "source": source_metadata.clone()
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id: second_project.id,
                chapter_id: first_chapter.id,
                workflow_mode: "first_frame_chain",
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: "电影感写实",
                duration_secs: 8,
                aspect_ratio: "16:9",
                reference_ids: &json!([]),
                generation_asset_fingerprint: Some(&asset_fingerprint),
                source_prompt_type: Some("frame_segment_plan"),
            },
            PromptOutputLanguage::ZhCn,
        )
        .await
        .unwrap();

    assert!(rows.is_empty());

    let rows = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id: first_project.id,
                chapter_id: first_chapter.id,
                workflow_mode: "first_frame_chain",
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: "电影感写实",
                duration_secs: 8,
                aspect_ratio: "16:9",
                reference_ids: &json!([]),
                generation_asset_fingerprint: Some(&asset_fingerprint),
                source_prompt_type: Some("frame_segment_plan"),
            },
            PromptOutputLanguage::ZhCn,
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].project_id, first_project.id);
    assert_eq!(rows[0].chapter_id, first_chapter.id);
}

#[tokio::test]
async fn matching_prompt_rows_rejects_metadata_that_conflicts_with_workflow_mode() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let source_metadata = json!({"content_sha256": "same-source"});
    let asset_fingerprint = test_generation_asset_fingerprint();
    let old_time = fixed_time(6);
    chapter_video_prompt::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("first_frame_chain".to_string()),
        shot_number: Set(1),
        title: Set(Some("模式列与元数据冲突".to_string())),
        visual_prompt: Set("这条记录不能被边界帧流程复用".to_string()),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        status: Set("pending".to_string()),
        metadata_json: Set(Some(json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "source": source_metadata.clone()
        }))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let rows = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id: project.id,
                chapter_id: chapter.id,
                workflow_mode: "boundary_frames",
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: "电影感写实",
                duration_secs: 8,
                aspect_ratio: "16:9",
                reference_ids: &json!([]),
                generation_asset_fingerprint: Some(&asset_fingerprint),
                source_prompt_type: Some("frame_segment_plan"),
            },
            PromptOutputLanguage::ZhCn,
        )
        .await
        .unwrap();

    assert!(
        rows.is_empty(),
        "metadata must not override chapter_video_prompt.workflow_mode"
    );
}
