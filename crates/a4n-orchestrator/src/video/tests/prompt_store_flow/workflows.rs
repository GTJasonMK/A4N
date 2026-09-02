#[tokio::test]
async fn sync_chapter_prompts_scopes_deletion_to_generated_prompt_types() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
        metadata_json: Set(Some(stable_prompt_metadata(
            json!({"source": {"content_sha256": "same"}}),
            1,
        ))),
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
                    workflow_mode: PromptWorkflowMode::SharedSegmentPlan,
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
                    metadata_json: Some(stable_prompt_metadata(
                        json!({"source": {"content_sha256": "same"}}),
                        1,
                    )),
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
    let service = crate::video::test_video_orchestrator(db.clone()).await;
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
        metadata_json: Set(Some(stable_prompt_metadata(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same-source"}
        }), 1))),
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
        metadata_json: Set(Some(stable_prompt_metadata(json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "same-source"}
        }), 9))),
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
                metadata_json: Some(stable_prompt_metadata(json!({
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same-source"}
                }), 1)),
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
                title: Some("冲突镜头"),
                scene_summary: Some("冲突摘要"),
                visual_prompt: "这条草稿显式属于边界帧链，但 metadata 声称首帧链",
                motion_prompt: Some("慢推"),
                negative_prompt: None,
                duration_secs: 8,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: None,
                metadata_json: Some(stable_prompt_metadata(json!({
                    "workflow_mode": "first_frame_chain",
                    "source_prompt_type": "frame_segment_plan",
                    "source": {"content_sha256": "same-source"}
                }), 1)),
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
