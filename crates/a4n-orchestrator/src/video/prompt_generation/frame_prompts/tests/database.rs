use super::*;

#[tokio::test]
async fn boundary_frame_image_lookup_accepts_root_project_images_for_video_project() {
    let db = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let root = create_project_record(
        db.as_ref(),
        CreateProjectRecordInput {
            user_id: 1,
            title: "根文本".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set("draft".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let video_project = create_derived_project_record(
        db.as_ref(),
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: PROJECT_KIND_VIDEO.to_string(),
            title: "视频项目".to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(video_project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("测试风格".to_string())),
        prompt_type: Set("frame_prompt".to_string()),
        workflow_mode: Set(PromptWorkflowMode::ChatVideo.as_str().to_string()),
        shot_number: Set(1),
        title: Set(Some("边界帧 1".to_string())),
        scene_summary: Set(Some("首帧".to_string())),
        visual_prompt: Set("首帧提示词".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(Some("低质量".to_string())),
        duration_secs: Set(5),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!([]))),
        status: Set("completed".to_string()),
        metadata_json: Set(Some(json!({
            "boundary_frame": frame(1, vec![])
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let image = generated_image::ActiveModel {
        project_id: Set(root.id),
        image_type: Set("video_boundary_frame".to_string()),
        file_path: Set("frames/1.png".to_string()),
        prompt: Set("首帧提示词".to_string()),
        model: Set("image-model".to_string()),
        width: Set(1280),
        height: Set(720),
        file_size: Set(None),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter.id)),
        panel_id: Set(None),
        metadata_json: Set(Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": prompt.id
            }
        }))),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let video_service = crate::video::test_video_orchestrator(db).await;

    let image_id_by_frame =
        boundary_frame_image_ids_by_number(&video_service, video_project.id, chapter.id, &[prompt])
            .await
            .unwrap();

    assert_eq!(image_id_by_frame.get(&1), Some(&image.id));
}
