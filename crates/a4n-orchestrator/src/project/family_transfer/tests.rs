use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use a4n_core::models::{
    media::media_asset,
    project::{novel_project, project_source_snapshot},
    user::user,
};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};

use crate::project::{
    ImportProjectPackageApplicationInput, PROJECT_FAMILY_PACKAGE_VERSION,
    ProjectPackageFileHostPort, TextCorePackage,
};
use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    FamilyImportMediaPayload, MediaFileEntry, ProjectFamilyExportRows, ProjectFamilyPackage,
    ProjectFamilyTransferHostPort, ReadProjectFamilyPackage, imported_project_family_media_path,
    project_family_export_media_files,
};

#[derive(Default)]
struct TestProjectFamilyTransferHost {
    render_calls: AtomicUsize,
    rendered_mode: Mutex<Option<String>>,
    rendered_project_ids: Mutex<Vec<i64>>,
    read_package: Mutex<Option<ReadProjectFamilyPackage>>,
    written_paths: Vec<String>,
    rollback_calls: AtomicUsize,
}

impl TestProjectFamilyTransferHost {
    fn with_read_package(read_package: ReadProjectFamilyPackage) -> Self {
        Self {
            read_package: Mutex::new(Some(read_package)),
            ..Self::default()
        }
    }
}

#[async_trait]
impl ProjectFamilyTransferHostPort for TestProjectFamilyTransferHost {
    async fn render_export_package(
        &self,
        package: &ProjectFamilyPackage,
    ) -> Result<Vec<u8>, String> {
        self.render_calls.fetch_add(1, Ordering::SeqCst);
        *self.rendered_mode.lock().expect("rendered mode lock") = Some(package.mode.clone());
        *self
            .rendered_project_ids
            .lock()
            .expect("rendered project ids lock") =
            package.projects.iter().map(|project| project.id).collect();
        Ok(vec![1, 2, 3])
    }

    async fn read_import_package(
        &self,
        _package_bytes: Vec<u8>,
    ) -> Result<ReadProjectFamilyPackage, String> {
        self.read_package
            .lock()
            .expect("read package lock")
            .take()
            .ok_or_else(|| "missing test package".to_string())
    }
}

#[async_trait]
impl ProjectPackageFileHostPort for TestProjectFamilyTransferHost {
    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        _bytes: &[u8],
        _label: &str,
    ) -> Result<(), String> {
        self.written_paths.push(relative_path.to_string());
        Ok(())
    }

    async fn rollback_imported_media_files(&mut self) -> Result<(), String> {
        self.rollback_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn root_export_application_enforces_owner_before_rows_assembly_and_render() {
    let (context, database) = setup().await;
    let owner_user_id = insert_active_user(database.as_ref(), "root-owner").await;
    let intruder_user_id = insert_active_user(database.as_ref(), "root-intruder").await;
    let root = insert_project(database.as_ref(), owner_user_id, "text", None).await;
    let host = TestProjectFamilyTransferHost::default();

    context
        .export_project_family_package_validated(
            ExportProjectFamilyPackageApplicationInput {
                user_id: intruder_user_id,
                root_project_id: root.id,
                include_derived_projects: false,
            },
            &host,
        )
        .await
        .expect_err("非 Owner 不得导出项目族包");
    assert_eq!(host.render_calls.load(Ordering::SeqCst), 0);

    let exported = context
        .export_project_family_package_validated(
            ExportProjectFamilyPackageApplicationInput {
                user_id: owner_user_id,
                root_project_id: root.id,
                include_derived_projects: false,
            },
            &host,
        )
        .await
        .expect("根项目族导出 application 应成功");

    assert_eq!(exported.project.id, root.id);
    assert_eq!(exported.package_bytes, vec![1, 2, 3]);
    assert_eq!(host.render_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        host.rendered_mode
            .lock()
            .expect("rendered mode lock")
            .as_deref(),
        Some("project_family")
    );
    assert_eq!(
        host.rendered_project_ids
            .lock()
            .expect("rendered project ids lock")
            .as_slice(),
        [root.id]
    );
}

#[tokio::test]
async fn derived_export_application_reuses_collaboration_root_owner_check() {
    let (context, database) = setup().await;
    let owner_user_id = insert_active_user(database.as_ref(), "derived-owner").await;
    let root = insert_project(database.as_ref(), owner_user_id, "text", None).await;
    let derived = insert_project(database.as_ref(), owner_user_id, "video", Some(root.id)).await;
    let host = TestProjectFamilyTransferHost::default();

    let exported = context
        .export_derived_project_package_validated(
            ExportDerivedProjectPackageApplicationInput {
                user_id: owner_user_id,
                derived_project_id: derived.id,
            },
            &host,
        )
        .await
        .expect("派生项目包导出 application 应成功");

    assert_eq!(exported.project.id, derived.id);
    assert_eq!(
        host.rendered_mode
            .lock()
            .expect("rendered mode lock")
            .as_deref(),
        Some("derived_project")
    );
    assert_eq!(
        host.rendered_project_ids
            .lock()
            .expect("rendered project ids lock")
            .as_slice(),
        [root.id, derived.id]
    );
}

#[tokio::test]
async fn import_application_owns_validation_database_import_and_user_projection() {
    let (context, _) = setup().await;
    let package = empty_package(project_model(101, 7, "text", None));
    let mut host = TestProjectFamilyTransferHost::with_read_package(ReadProjectFamilyPackage {
        package,
        media_payload: empty_media_payload(),
    });

    let imported = context
        .import_project_family_package_validated(
            ImportProjectPackageApplicationInput {
                user_id: 29,
                package_bytes: vec![9],
            },
            &mut host,
        )
        .await
        .expect("项目族导入 application 应成功");

    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].user_id, 29);
    assert_eq!(imported[0].project_kind, "text");
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn import_application_rejects_invalid_package_before_file_rollback() {
    let (context, _) = setup().await;
    let mut package = empty_package(project_model(201, 9, "text", None));
    package.version = "invalid-version".to_string();
    let mut host = TestProjectFamilyTransferHost::with_read_package(ReadProjectFamilyPackage {
        package,
        media_payload: empty_media_payload(),
    });

    let error = context
        .import_project_family_package_validated(
            ImportProjectPackageApplicationInput {
                user_id: 31,
                package_bytes: vec![8],
            },
            &mut host,
        )
        .await
        .expect_err("非法项目族包必须失败");

    assert!(error.to_string().contains("版本"));
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn media_index_and_import_path_preserve_field_and_extension() {
    let mut rows = ProjectFamilyExportRows::default();
    rows.media_assets.push(media_asset::Model {
        id: 10,
        project_id: 2,
        root_project_id: 1,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "generated".to_string(),
        file_path: Some("projects/2/media/video/source.mp4".to_string()),
        thumbnail_path: Some("projects/2/media/video/thumb.png".to_string()),
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        file_size: None,
        checksum: None,
        status: "completed".to_string(),
        error_message: None,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    });

    let media_files = project_family_export_media_files(&rows).expect("媒体索引应可组装");
    assert_eq!(media_files.len(), 2);
    assert!(
        media_files
            .iter()
            .any(|entry| entry.package_path == "media/10/file.mp4")
    );
    assert!(
        media_files
            .iter()
            .any(|entry| entry.package_path == "media/10/thumbnail.png")
    );
    assert_eq!(
        imported_project_family_media_path(
            20,
            "video",
            30,
            a4n_core::models::media::media_asset::THUMBNAIL_PATH_FIELD,
            "media/x.png",
        )
            .expect("导入路径应可生成"),
        "projects/20/media/video/imported_package_30_thumbnail.png"
    );
}

async fn setup() -> (OrchestratorContext, Arc<a4n_db::DatabaseConnection>) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (context, database)
}

async fn insert_active_user(database: &a4n_db::DatabaseConnection, label: &str) -> i64 {
    let now = Utc::now().naive_utc();
    user::ActiveModel {
        username: Set(format!("family-transfer-{label}")),
        email: Set(format!("family-transfer-{label}@example.test")),
        password_hash: Set("test-password-hash".to_string()),
        role: Set("user".to_string()),
        oauth_provider: Set(None),
        oauth_subject: Set(None),
        avatar_url: Set(None),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试用户应可写入")
    .id
}

async fn insert_project(
    database: &a4n_db::DatabaseConnection,
    user_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
) -> novel_project::Model {
    let now = Utc::now().naive_utc();
    let source_snapshot_id = if let Some(root_project_id) = root_project_id {
        Some(
            project_source_snapshot::ActiveModel {
                root_project_id: Set(root_project_id),
                source_version_hash: Set(format!("family-transfer-snapshot-{root_project_id}")),
                blueprint_version_id: Set(None),
                chapter_scope_json: Set(serde_json::json!({
                    "start_chapter": 1,
                    "end_chapter": 1
                })),
                chapter_hashes_json: Set(serde_json::json!([])),
                profile_hashes_json: Set(serde_json::json!({})),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(database)
            .await
            .expect("测试源快照应可写入")
            .id,
        )
    } else {
        None
    };
    novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(format!("family-transfer-{user_id}-{project_kind}")),
        description: Set("project family transfer application test".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set(project_kind.to_string()),
        root_project_id: Set(root_project_id),
        source_snapshot_id: Set(source_snapshot_id),
        source_sync_status: Set("current".to_string()),
        status: Set("draft".to_string()),
        target_chapters: Set(12),
        current_chapter: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试项目应可写入")
}

fn project_model(
    id: i64,
    user_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
) -> novel_project::Model {
    let now = Utc::now().naive_utc();
    novel_project::Model {
        id,
        user_id,
        title: format!("import-{id}"),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: project_kind.to_string(),
        root_project_id,
        source_snapshot_id: root_project_id.map(|_| 901),
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 12,
        current_chapter: 0,
        total_chapters: Some(12),
        chapters_per_part: Some(6),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    }
}

fn empty_package(root: novel_project::Model) -> ProjectFamilyPackage {
    ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: "project_family".to_string(),
        root_project_id: root.id,
        projects: vec![root],
        text_core: TextCorePackage::default(),
        source_snapshots: Vec::new(),
        source_snapshot_chapters: Vec::new(),
        source_snapshot_outlines: Vec::new(),
        source_snapshot_profiles: Vec::new(),
        text_anchors: Vec::new(),
        profile_evidence: Vec::new(),
        generation_specs: Vec::new(),
        generation_spec_text_anchors: Vec::new(),
        generation_spec_entity_refs: Vec::new(),
        generation_spec_media_refs: Vec::new(),
        prompt_artifacts: Vec::new(),
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        galgame_scripts: Vec::new(),
        galgame_scenes: Vec::new(),
        galgame_lines: Vec::new(),
        galgame_choices: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
        source_sync_runs: Vec::new(),
        source_sync_conflicts: Vec::new(),
        source_sync_decisions: Vec::new(),
        media_files: Vec::<MediaFileEntry>::new(),
    }
}

fn empty_media_payload() -> FamilyImportMediaPayload {
    FamilyImportMediaPayload {
        entries: Default::default(),
        bytes: Default::default(),
    }
}
