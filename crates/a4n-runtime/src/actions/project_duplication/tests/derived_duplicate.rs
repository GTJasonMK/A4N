#[tokio::test]
async fn duplicate_derived_project_record_copies_snapshot_without_copying_root_chapters() {
    let db = setup_db().await;
    let root = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    let root_chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("源章节".to_string())),
        content: Set("根文本章节正文。".to_string()),
        word_count: Set(8),
        status: Set("completed".to_string()),
        summary: Set(Some("根文本摘要".to_string())),
        style_tags: Set(None),
        analysis_data: Set(Some(json!({"k": "v"}))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root.id),
        source_version_hash: Set("source-hash".to_string()),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(json!({"start": 1, "end": 1})),
        chapter_hashes_json: Set(json!([{"chapter_number": 1}])),
        profile_hashes_json: Set(json!([])),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let snapshot_chapter = source_snapshot_chapter::ActiveModel {
        snapshot_id: Set(snapshot.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(root_chapter.id),
        chapter_number: Set(1),
        title: Set(Some("源章节".to_string())),
        content: Set(root_chapter.content.clone()),
        summary: Set(root_chapter.summary.clone()),
        analysis_data_json: Set(root_chapter.analysis_data.clone()),
        content_hash: Set("content-hash".to_string()),
        analysis_hash: Set(Some("analysis-hash".to_string())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let derived = novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("视频项目".to_string()),
        description: Set("派生视频".to_string()),
        project_type: Set(root.project_type.clone()),
        project_kind: Set("video".to_string()),
        root_project_id: Set(Some(root.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_sync_status: Set("current".to_string()),
        status: Set(ProjectStatus::Draft.as_str().to_string()),
        target_chapters: Set(1),
        current_chapter: Set(0),
        total_chapters: Set(Some(1)),
        chapters_per_part: Set(root.chapters_per_part),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let anchor = text_anchor::ActiveModel {
        project_id: Set(root.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        source_kind: Set("chapter_content".to_string()),
        source_id: Set(snapshot_chapter.id),
        paragraph_index: Set(0),
        start_offset: Set(0),
        end_offset: Set(6),
        quote: Set("根文本章节正文。".to_string()),
        quote_hash: Set("quote-hash".to_string()),
        source_version_hash: Set("content-hash".to_string()),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let segment = video_segment::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        segment_type: Set("normal".to_string()),
        order_index: Set(1),
        title: Set("镜头 1".to_string()),
        duration_secs: Set(6.0),
        source_generation_spec_id: Set(None),
        result_media_asset_id: Set(None),
        previous_segment_id: Set(None),
        next_segment_id: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let spec = generation_spec::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        target_kind: Set("video_segment".to_string()),
        target_entity_kind: Set("segment".to_string()),
        target_entity_id: Set(Some(segment.id)),
        status: Set("draft".to_string()),
        source_version_hash: Set("source-hash".to_string()),
        params_json: Set(json!({"duration_secs": 6})),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    generation_spec_text_anchor::ActiveModel {
        generation_spec_id: Set(spec.id),
        text_anchor_id: Set(anchor.id),
        role: Set("direct_content".to_string()),
    }
    .insert(&db)
    .await
    .unwrap();
    let prompt = prompt_artifact::ActiveModel {
        generation_spec_id: Set(spec.id),
        prompt_kind: Set("video_prompt".to_string()),
        prompt_text: Set("镜头自然语言提示词".to_string()),
        renderer_version: Set("test".to_string()),
        edited_by_user: Set(true),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let test_data_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/test-data");
    std::fs::create_dir_all(&test_data_root).unwrap();
    let data_dir = tempfile::Builder::new()
        .prefix("a4n_duplication_media_")
        .tempdir_in(test_data_root)
        .unwrap();
    let media_dir = data_dir.path().join("source");
    std::fs::create_dir_all(&media_dir).unwrap();
    let segment_relative = "source/segment.mp4".to_string();
    let preview_relative = "source/preview.mp4".to_string();
    std::fs::write(data_dir.path().join(&segment_relative), b"segment").unwrap();
    std::fs::write(data_dir.path().join(&preview_relative), b"preview").unwrap();
    let asset = media_asset::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        media_kind: Set("video".to_string()),
        origin: Set("imported".to_string()),
        file_path: Set(Some(segment_relative)),
        thumbnail_path: Set(None),
        width: Set(None),
        height: Set(None),
        duration_secs: Set(Some(6.0)),
        fps: Set(None),
        file_size: Set(None),
        checksum: Set(None),
        status: Set("completed".to_string()),
        error_message: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let preview_asset = media_asset::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        media_kind: Set("video".to_string()),
        origin: Set("assembled_preview".to_string()),
        file_path: Set(Some(preview_relative)),
        thumbnail_path: Set(None),
        width: Set(None),
        height: Set(None),
        duration_secs: Set(Some(6.0)),
        fps: Set(None),
        file_size: Set(None),
        checksum: Set(None),
        status: Set("completed".to_string()),
        error_message: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(Some(spec.id)),
        prompt_artifact_id: Set(Some(prompt.id)),
        provider_job_id: Set(None),
        source_media_asset_id: Set(None),
        source_external_path: Set(None),
        imported_from_user: Set(true),
        request_snapshot_json: Set(Some(json!({"provider": "test"}))),
        created_at: Set(fixed_time()),
    }
    .insert(&db)
    .await
    .unwrap();
    a4n_core::models::generation::generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(spec.id),
        media_asset_id: Set(asset.id),
        role: Set("style_reference".to_string()),
    }
    .insert(&db)
    .await
    .unwrap();
    video_segment::ActiveModel {
        id: Set(segment.id),
        source_generation_spec_id: Set(Some(spec.id)),
        result_media_asset_id: Set(Some(asset.id)),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    media_binding::ActiveModel {
        media_asset_id: Set(asset.id),
        target_kind: Set("video_segment".to_string()),
        target_id: Set(segment.id),
        role: Set("generated_result".to_string()),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let timeline = assembly_timeline::ActiveModel {
        project_id: Set(derived.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        version: Set(1),
        status: Set("preview_ready".to_string()),
        preview_media_asset_id: Set(Some(preview_asset.id)),
        preview_cache_key: Set(Some("cache-key".to_string())),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assembly_clip::ActiveModel {
        timeline_id: Set(timeline.id),
        media_asset_id: Set(asset.id),
        order_index: Set(1),
        trim_start_secs: Set(0.0),
        trim_end_secs: Set(6.0),
        volume: Set(1.0),
        transition_kind: Set("cut".to_string()),
        source_segment_id: Set(Some(segment.id)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let mut copied_media_files = Vec::new();
    let mut media_file_copier = RuntimeProjectMediaFileCopier {
        data_dir: data_dir.path(),
        copied_media_files: &mut copied_media_files,
    };
    let duplicated =
        duplicate_derived_project_record(&db, &derived, 1, "视频项目 副本", &mut media_file_copier)
            .await
            .expect("derived project should duplicate snapshot");

    let duplicated_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let duplicated_snapshot_id = duplicated
        .source_snapshot_id
        .expect("duplicated project should reference snapshot");
    let duplicated_snapshot_chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(duplicated_snapshot_id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(duplicated.project_kind, "video");
    assert_eq!(duplicated.root_project_id, Some(root.id));
    assert_ne!(duplicated_snapshot_id, snapshot.id);
    assert!(duplicated_chapters.is_empty());
    assert_eq!(duplicated_snapshot_chapters.len(), 1);
    assert_eq!(
        duplicated_snapshot_chapters[0].content,
        root_chapter.content
    );

    let copied_segment = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("video segment should be copied");
    let copied_spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("generation spec should be copied");
    let copied_assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let copied_asset = copied_assets
        .iter()
        .find(|asset| asset.origin == "imported")
        .expect("segment media asset should be copied");
    let copied_preview_asset = copied_assets
        .iter()
        .find(|asset| asset.origin == "assembled_preview")
        .expect("preview media asset should be copied");
    let copied_prompt = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(copied_spec.id))
        .one(&db)
        .await
        .unwrap()
        .expect("prompt artifact should be copied");
    let copied_anchor_ref = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(copied_spec.id))
        .one(&db)
        .await
        .unwrap()
        .expect("text anchor ref should be copied");
    let copied_anchor = text_anchor::Entity::find_by_id(copied_anchor_ref.text_anchor_id)
        .one(&db)
        .await
        .unwrap()
        .expect("text anchor should be copied");
    let copied_provenance = media_provenance::Entity::find_by_id(copied_asset.id)
        .one(&db)
        .await
        .unwrap()
        .expect("media provenance should be copied");
    let copied_timeline = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("assembly timeline should be copied");
    let copied_clip = assembly_clip::Entity::find()
        .filter(assembly_clip::Column::TimelineId.eq(copied_timeline.id))
        .one(&db)
        .await
        .unwrap()
        .expect("assembly clip should be copied");
    let copied_binding = media_binding::Entity::find()
        .filter(media_binding::Column::MediaAssetId.eq(copied_asset.id))
        .one(&db)
        .await
        .unwrap()
        .expect("media binding should be copied");

    assert_eq!(copied_spec.source_snapshot_id, Some(duplicated_snapshot_id));
    assert_eq!(
        copied_spec.source_snapshot_chapter_id,
        Some(duplicated_snapshot_chapters[0].id)
    );
    assert_eq!(copied_spec.target_entity_id, Some(copied_segment.id));
    assert_eq!(
        copied_segment.source_generation_spec_id,
        Some(copied_spec.id)
    );
    assert_eq!(copied_segment.result_media_asset_id, Some(copied_asset.id));
    assert_eq!(copied_prompt.prompt_text, "镜头自然语言提示词");
    assert!(copied_prompt.edited_by_user);
    assert_eq!(
        copied_anchor.source_snapshot_id,
        Some(duplicated_snapshot_id)
    );
    assert_eq!(
        copied_anchor.source_snapshot_chapter_id,
        Some(duplicated_snapshot_chapters[0].id)
    );
    assert_eq!(copied_provenance.generation_spec_id, Some(copied_spec.id));
    assert_eq!(copied_provenance.prompt_artifact_id, Some(copied_prompt.id));
    assert_eq!(
        copied_timeline.preview_media_asset_id,
        Some(copied_preview_asset.id)
    );
    assert_eq!(copied_clip.media_asset_id, copied_asset.id);
    assert_eq!(copied_clip.source_segment_id, Some(copied_segment.id));
    assert_eq!(copied_binding.target_id, copied_segment.id);
}
