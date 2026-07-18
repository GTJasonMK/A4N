#[tokio::test]
async fn matching_prompt_rows_finds_first_frame_video_prompt_by_workflow_contract() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
                video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
                video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
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
                video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
                video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
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
                video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
                video_execution_identity: VideoExecutionIdentityMatch::NotApplicable,
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
