#[tokio::test]
async fn sync_chapter_prompts_preserves_unchanged_video_prompt_identity() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
        metadata_json: Set(Some(stable_prompt_metadata(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"}
        }), 1))),
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
        metadata_json: Set(Some(stable_prompt_metadata(
            json!({"source": {"content_sha256": "same"}}),
            9,
        ))),
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
                    metadata_json: Some(stable_prompt_metadata(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"}
                    }), 1)),
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
                    metadata_json: Some(stable_prompt_metadata(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"}
                    }), 2)),
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
        metadata_json: Set(Some(stable_prompt_metadata(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "prompt_output_language": "zh_cn",
            "generation_asset_fingerprint": test_generation_asset_fingerprint()
        }), 1))),
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
                    metadata_json: Some(stable_prompt_metadata(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"},
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "prompt_output_language": "zh_cn",
                        "generation_asset_fingerprint": test_generation_asset_fingerprint()
                    }), 1)),
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
                    metadata_json: Some(stable_prompt_metadata(json!({
                        "source_prompt_type": "frame_segment_plan",
                        "source": {"content_sha256": "same"},
                        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                        "prompt_output_language": "zh_cn",
                        "generation_asset_fingerprint": test_generation_asset_fingerprint()
                    }), 2)),
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
        metadata_json: Set(Some(stable_prompt_metadata(json!({
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
        }), 1))),
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
                "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
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
                metadata_json: Some(stable_prompt_metadata(json!({
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
                }), 1)),
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
