use super::*;
use a4n_core::models::{
    media::media_asset,
    project::{blueprint_character, blueprint_version, novel_blueprint, novel_project},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::error::Error;

struct FailingMediaImporter;

impl ProjectFamilyMediaFileImporter for FailingMediaImporter {
    fn write_imported_media_field(
        &mut self,
        _old_asset: &media_asset::Model,
        _new_asset: &media_asset::Model,
        _field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        Err(ProjectFamilyMediaImportError::Write(
            "test media write failure".to_string(),
        ))
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
        a4n_core::models::user::role::ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: 99,
        title: "生命周期门禁导入".to_string(),
        description: String::new(),
        project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        project_kind: crate::project::PROJECT_KIND_TEXT.to_string(),
        source_mode: crate::project::SOURCE_MODE_SELF.to_string(),
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
        projects: vec![project],
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
        a4n_core::models::user::role::ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: 99,
        title: "雾城".to_string(),
        description: String::new(),
        project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        project_kind: crate::project::PROJECT_KIND_TEXT.to_string(),
        source_mode: crate::project::SOURCE_MODE_SELF.to_string(),
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
        projects: vec![project],
        text_core: crate::project::ProjectFamilyTextCoreRows {
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
}

#[tokio::test]
async fn project_family_media_write_failure_rolls_back_database_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = crate::auth::create_user(
        &db,
        "project_family_media_failure",
        "project-family-media-failure@example.invalid",
        "project-family-media-failure-password",
        a4n_core::models::user::role::ROLE_USER,
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let project = novel_project::Model {
        id: 1,
        user_id: user.id,
        title: "媒体失败回滚".to_string(),
        description: String::new(),
        project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        project_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
        source_mode: crate::project::SOURCE_MODE_FREEFORM.to_string(),
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
        projects: vec![project],
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
