use super::*;
use a4n_db_models::models::{
    ai::generated_video,
    chapter::{chapter, chapter_video_prompt},
    media::media_asset,
    project::{blueprint_character, blueprint_version, novel_blueprint, novel_project},
    scene::scene_profile,
};
use a4n_domain::auth::ROLE_USER;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::error::Error;

mod fact_feedback;
mod remap;

struct FailingMediaImporter;

impl ProjectFamilyMediaFileImporter for FailingMediaImporter {
    fn write_imported_media_field(
        &mut self,
        _project_id: i64,
        _owner_kind: &str,
        _old_owner_id: i64,
        _new_owner_id: i64,
        _field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        Err(ProjectFamilyMediaImportError::Write(
            "test media write failure".to_string(),
        ))
    }
}

struct EmptyMediaImporter;

impl ProjectFamilyMediaFileImporter for EmptyMediaImporter {
    fn write_imported_media_field(
        &mut self,
        _project_id: i64,
        _owner_kind: &str,
        _old_owner_id: i64,
        _new_owner_id: i64,
        _field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        Ok(None)
    }
}

#[test]
fn project_family_import_database_error_keeps_db_source() {
    let error = ProjectFamilyImportError::Database(DbErr::Custom("begin failed".to_string()));

    assert_eq!(error.to_string(), "Custom Error: begin failed");
    assert!(matches!(
        error,
        ProjectFamilyImportError::Database(DbErr::Custom(message))
            if message == "begin failed"
    ));
}

#[test]
fn project_family_import_commit_error_keeps_context_and_db_source() {
    let error = ProjectFamilyImportError::Commit(DbErr::Custom("commit failed".to_string()));

    assert_eq!(
        error.to_string(),
        "提交项目族导入事务失败: Custom Error: commit failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn project_family_import_invalid_package_has_no_db_source() {
    let error = ProjectFamilyImportError::InvalidPackage("映射缺失".to_string());

    assert_eq!(error.to_string(), "映射缺失");
    assert!(error.source().is_none());
}

#[test]
fn project_family_import_context_error_keeps_operation_and_db_source() {
    let error = ProjectFamilyImportError::DatabaseContext {
        context: "导入测试记录失败",
        source: DbErr::Custom("insert failed".to_string()),
    };

    assert_eq!(
        error.to_string(),
        "导入测试记录失败: Custom Error: insert failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn project_family_import_media_error_has_distinct_classification() {
    let error = ProjectFamilyImportError::MediaImport(ProjectFamilyMediaImportError::Write(
        "write failed".to_string(),
    ));

    assert_eq!(error.to_string(), "write failed");
    assert!(matches!(
        error,
        ProjectFamilyImportError::MediaImport(ProjectFamilyMediaImportError::Write(message))
            if message == "write failed"
    ));
}

#[tokio::test]
async fn project_family_import_activates_deserialized_project_lifecycle_gate() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = crate::auth::create_user(
        &db,
        "project_family_lifecycle_gate",
        "project-family-lifecycle-gate@example.invalid",
        "project-family-lifecycle-gate-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: 99,
        title: "生命周期门禁导入".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: crate::project::PROJECT_KIND_TEXT.to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: String::new(),
        target_chapters: 1,
        current_chapter: 0,
        total_chapters: Some(1),
        chapters_per_part: Some(1),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    };
    let package = ProjectFamilyImportPackage {
        root_project_id: project.id,
        projects: vec![crate::project::project_record_from_model(project.clone())],
        ..Default::default()
    };

    let imported =
        import_project_family_package_rows(&db, user.id, &package, &mut FailingMediaImporter)
            .await
            .expect("deserialized package project should import as active");

    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].user_id, user.id);
    assert_eq!(imported[0].lifecycle_gate, "active");
}

#[tokio::test]
async fn project_family_import_rebuilds_raw_blueprint_from_formal_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = crate::auth::create_user(
        &db,
        "project_family_blueprint",
        "project-family-blueprint@example.invalid",
        "project-family-blueprint-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: 99,
        title: "雾城".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: crate::project::PROJECT_KIND_TEXT.to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        status: "blueprint_ready".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 30,
        current_chapter: 0,
        total_chapters: Some(30),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: Some("{\"untrusted\":true}".to_string()),
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    };
    let blueprint = novel_blueprint::Model {
        id: 2,
        project_id: project.id,
        title: project.title.clone(),
        genre: "悬疑".to_string(),
        style: Some("冷峻".to_string()),
        tone: None,
        target_audience: None,
        one_sentence_summary: None,
        full_synopsis: Some("追查倒流迷雾".to_string()),
        world_setting: Some(serde_json::json!({"premise": "迷雾倒流"})),
        current_version: "1.0".to_string(),
        needs_part_outlines: false,
        total_chapters: Some(30),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: Some("fox".to_string()),
        created_at: now,
        updated_at: now,
    };
    let character = blueprint_character::Model {
        id: 3,
        project_id: project.id,
        name: "林澈".to_string(),
        character_type: "protagonist".to_string(),
        gender: None,
        age: None,
        appearance: None,
        personality: Some("克制".to_string()),
        background: None,
        motivation: None,
        arc_summary: None,
        relationship_to_protagonist: None,
        arc_stages: None,
        created_at: now,
        updated_at: now,
    };
    let snapshot = crate::blueprint::blueprint_version_snapshot(
        &blueprint,
        std::slice::from_ref(&character),
        &[],
    );
    let package = ProjectFamilyImportPackage {
        root_project_id: project.id,
        projects: vec![crate::project::project_record_from_model(project.clone())],
        text_core: crate::project::ProjectFamilyTextCoreRows {
            fact_feedback: None,
            blueprint: Some(blueprint),
            blueprint_versions: vec![blueprint_version::Model {
                id: 4,
                project_id: 1,
                version: "1.0".to_string(),
                change_description: Some("正式版本".to_string()),
                blueprint_data: snapshot.clone(),
                created_at: now,
                created_by: Some("system".to_string()),
            }],
            blueprint_characters: vec![character],
            ..Default::default()
        },
        ..Default::default()
    };

    let imported =
        import_project_family_package_rows(&db, user.id, &package, &mut FailingMediaImporter)
            .await
            .expect("canonical blueprint package should import");
    let imported_project = novel_project::Entity::find_by_id(imported[0].id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let raw: serde_json::Value =
        serde_json::from_str(imported_project.raw_blueprint.as_deref().unwrap()).unwrap();

    assert_eq!(raw, snapshot);
    assert!(raw.get("untrusted").is_none());

    let mut history_only = package;
    let live_character = history_only.text_core.blueprint_characters[0].clone();
    history_only.text_core.blueprint = None;
    history_only.text_core.blueprint_characters.clear();
    history_only.text_core.blueprint_relationships.clear();
    let imported =
        import_project_family_package_rows(&db, user.id, &history_only, &mut EmptyMediaImporter)
            .await
            .expect("reset projects retain independently valid blueprint history");
    let restored = novel_project::Entity::find_by_id(imported[0].id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(restored.raw_blueprint.is_none());
    assert!(
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(restored.id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(restored.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].blueprint_data, snapshot);

    history_only
        .text_core
        .blueprint_characters
        .push(live_character);
    let error =
        import_project_family_package_rows(&db, user.id, &history_only, &mut EmptyMediaImporter)
            .await
            .expect_err("live blueprint rows still require their current blueprint");
    assert!(error.to_string().contains("包含当前蓝图角色或关系"));
    history_only.text_core.blueprint_characters.clear();
    history_only.text_core.blueprint_versions[0].blueprint_data =
        serde_json::json!({"invalid": true});
    let error =
        import_project_family_package_rows(&db, user.id, &history_only, &mut EmptyMediaImporter)
            .await
            .expect_err("retained history must still satisfy the full snapshot contract");
    assert!(error.to_string().contains("蓝图版本快照无效"));
}

#[tokio::test]
async fn project_family_media_write_failure_rolls_back_database_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = crate::auth::create_user(
        &db,
        "project_family_media_failure",
        "project-family-media-failure@example.invalid",
        "project-family-media-failure-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: user.id,
        title: "媒体失败回滚".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 1,
        current_chapter: 0,
        total_chapters: Some(1),
        chapters_per_part: Some(1),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    };
    let asset = media_asset::Model {
        id: 10,
        project_id: project.id,
        root_project_id: project.id,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "image".to_string(),
        origin: "imported".to_string(),
        file_path: Some("projects/1/images/imported.png".to_string()),
        thumbnail_path: None,
        width: Some(1),
        height: Some(1),
        duration_secs: None,
        fps: None,
        file_size: Some(1),
        checksum: None,
        status: "completed".to_string(),
        error_message: None,
        created_at: now,
        updated_at: now,
    };
    let package = ProjectFamilyImportPackage {
        root_project_id: project.id,
        projects: vec![crate::project::project_record_from_model(project.clone())],
        media_assets: vec![asset],
        ..Default::default()
    };

    let error =
        import_project_family_package_rows(&db, user.id, &package, &mut FailingMediaImporter)
            .await
            .expect_err("media write failure must abort the import transaction");

    assert!(
        matches!(
            error,
            ProjectFamilyImportError::MediaImport(ProjectFamilyMediaImportError::Write(ref message))
                if message == "test media write failure"
        ),
        "unexpected import error: {error:?}"
    );
    assert!(
        novel_project::Entity::find()
            .filter(novel_project::Column::UserId.eq(user.id))
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        media_asset::Entity::find()
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn project_family_import_batches_video_reference_fixups() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = crate::auth::create_user(
        &db,
        "project_family_video_fixups",
        "project-family-video-fixups@example.invalid",
        "project-family-video-fixups-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: 99,
        title: "视频引用回填".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 1,
        current_chapter: 0,
        total_chapters: Some(1),
        chapters_per_part: Some(1),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    };
    let chapter = chapter::Model {
        id: 2,
        project_id: project.id,
        outline_id: None,
        chapter_number: 1,
        title: Some("第一章".to_string()),
        content: "视频正文".to_string(),
        word_count: 4,
        status: chapter::STATUS_COMPLETED.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    };
    let project_id = project.id;
    let chapter_id = chapter.id;
    let scene = |id: i64, name: &str, parent_scene_id| scene_profile::Model {
        id,
        project_id,
        ownership_kind: a4n_domain::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC.to_string(),
        name: name.to_string(),
        normalized_name: name.to_string(),
        aliases_json: serde_json::json!([]),
        scene_type: "interior".to_string(),
        importance: "primary".to_string(),
        parent_scene_id,
        base_description: name.to_string(),
        visual_description: "布景".to_string(),
        spatial_layout: "方形".to_string(),
        atmosphere: "安静".to_string(),
        entry_points_json: serde_json::json!([]),
        rules_json: serde_json::json!([]),
        current_state_json: serde_json::json!({}),
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: serde_json::json!([]),
        image_prompt_seed: name.to_string(),
        created_at: now,
        updated_at: now,
    };
    let prompt = |id: i64, shot_number: i32, generated_video_id: i64, metadata_json| {
        ChapterVideoPromptRecord {
            id,
            project_id,
            chapter_id,
            style: Some("电影感".to_string()),
            prompt_type: chapter_video_prompt::PROMPT_TYPE_VIDEO.to_string(),
            workflow_mode: chapter_video_prompt::WORKFLOW_MODE_STORYBOARD.to_string(),
            shot_number,
            title: Some(format!("镜头 {shot_number}")),
            scene_summary: Some("场景".to_string()),
            visual_prompt: format!("画面 {shot_number}"),
            motion_prompt: Some("移动".to_string()),
            negative_prompt: None,
            duration_secs: 2,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: chapter_video_prompt::STATUS_COMPLETED.to_string(),
            generated_video_id: Some(generated_video_id),
            timeline_item_id: Some(900 + id),
            metadata_json,
            created_at: now,
            updated_at: now,
        }
    };
    let video = |id: i64, prompt: &str, source_video_id, metadata_json| generated_video::Model {
        id,
        project_id,
        chapter_id: Some(chapter_id),
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
        workflow_mode: Some("storyboard".to_string()),
        file_path: None,
        thumbnail_path: None,
        prompt: prompt.to_string(),
        negative_prompt: None,
        model: "test-video".to_string(),
        width: 640,
        height: 360,
        duration_secs: 2.0,
        fps: 24,
        file_size: None,
        source_video_id,
        source_image_id: None,
        status: a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string(),
        provider_job_id: Some(format!("old-job-{id}")),
        error_message: None,
        timeline_item_id: Some(800 + id),
        metadata_json,
        is_favorite: false,
        created_at: now,
        updated_at: now,
    };
    let package = ProjectFamilyImportPackage {
        root_project_id: project.id,
        projects: vec![crate::project::project_record_from_model(project.clone())],
        text_core: crate::project::ProjectFamilyTextCoreRows {
            fact_feedback: None,
            chapters: vec![chapter],
            scene_profiles: vec![scene(30, "大厅", None), scene(31, "内室", Some(30))],
            ..Default::default()
        },
        chapter_video_prompts: vec![
            prompt(
                20,
                1,
                10,
                Some(serde_json::json!({
                    "generated_video_id": 11,
                    "prompt_output_language": "zh_cn"
                })),
            ),
            prompt(
                21,
                2,
                11,
                Some(serde_json::json!({"prompt_output_language": "en"})),
            ),
        ],
        generated_videos: vec![
            video(10, "第一段", None, None),
            video(
                11,
                "第二段",
                Some(10),
                Some(serde_json::json!({"source_video_id": 10})),
            ),
        ],
        ..Default::default()
    };

    let imported =
        import_project_family_package_rows(&db, user.id, &package, &mut EmptyMediaImporter)
            .await
            .unwrap();
    let imported_videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(imported[0].id))
        .order_by_asc(generated_video::Column::Prompt)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(imported_videos.len(), 2);
    assert_eq!(imported_videos[0].source_video_id, None);
    assert_eq!(imported_videos[0].provider_job_id, None);
    assert_eq!(imported_videos[0].timeline_item_id, None);
    assert_eq!(
        imported_videos[1].source_video_id,
        Some(imported_videos[0].id)
    );
    assert_eq!(
        imported_videos[1].metadata_json,
        Some(serde_json::json!({"source_video_id": imported_videos[0].id}))
    );

    let imported_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(imported[0].id))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(imported_prompts.len(), 2);
    assert_eq!(
        imported_prompts[0].generated_video_id,
        Some(imported_videos[0].id)
    );
    assert_eq!(imported_prompts[0].timeline_item_id, None);
    assert_eq!(
        imported_prompts[0].metadata_json,
        Some(serde_json::json!({
            "generated_video_id": imported_videos[1].id,
            "prompt_output_language": "zh_cn"
        }))
    );

    let imported_scenes = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(imported[0].id))
        .order_by_asc(scene_profile::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(imported_scenes.len(), 2);
    assert_eq!(imported_scenes[0].parent_scene_id, None);
    assert_eq!(
        imported_scenes[1].parent_scene_id,
        Some(imported_scenes[0].id)
    );
}
