use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use a4n_api_contract::project_transfer::{
    ExportVideoStudioProjectApplicationInput, ImportProjectPackageApplicationInput,
    ProjectPackageExportInput, ProjectPackageImportPayload, ProjectPackageTransferHostPort,
};
use a4n_db::project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record};
use a4n_db_models::models::media::media_asset::FILE_PATH_FIELD;
use a4n_db_models::models::{media::media_asset, user::user};
use a4n_domain::CanvasNodeRefKind;
use async_trait::async_trait;
use sea_orm::ActiveModelTrait;

use crate::project::{
    ImportMediaPayload, MediaFileEntry, PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION,
    VideoStudioProjectPackage, media_file_key,
};
use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::media_package_path;

struct ReadVideoStudioProjectPackage {
    package: VideoStudioProjectPackage,
    media_payload: ImportMediaPayload,
}

impl ReadVideoStudioProjectPackage {
    fn into_host_payload(self) -> ProjectPackageImportPayload {
        ProjectPackageImportPayload {
            manifest: serde_json::to_value(self.package).expect("serialize test package"),
            media_bytes: self.media_payload.bytes,
        }
    }
}

#[derive(Default)]
struct TestVideoStudioTransferHost {
    render_calls: AtomicUsize,
    read_calls: AtomicUsize,
    write_calls: AtomicUsize,
    rollback_calls: AtomicUsize,
    written_paths: Mutex<Vec<String>>,
    read_package: Mutex<Option<ProjectPackageImportPayload>>,
    rendered_package: Mutex<Option<VideoStudioProjectPackage>>,
}

impl TestVideoStudioTransferHost {
    fn with_read_package(read_package: ReadVideoStudioProjectPackage) -> Self {
        Self {
            read_package: Mutex::new(Some(read_package.into_host_payload())),
            ..Self::default()
        }
    }
}

#[async_trait]
impl ProjectPackageTransferHostPort for TestVideoStudioTransferHost {
    async fn rollback_imported_project(
        &self,
        _user_id: i64,
        _project_id: i64,
    ) -> Result<(), String> {
        panic!("standalone canvas import must roll back its transaction");
    }

    async fn render_export_package(
        &self,
        input: ProjectPackageExportInput,
    ) -> Result<Vec<u8>, String> {
        let package: VideoStudioProjectPackage =
            serde_json::from_value(input.manifest).map_err(|error| error.to_string())?;
        self.render_calls.fetch_add(1, Ordering::SeqCst);
        *self.rendered_package.lock().expect("rendered package lock") = Some(package.clone());
        Ok(vec![1, 2, 3])
    }

    async fn read_import_package(
        &self,
        _package_bytes: Vec<u8>,
        _manifest_path: &str,
        _package_label: &str,
        _manifest_label: &str,
    ) -> Result<ProjectPackageImportPayload, String> {
        self.read_calls.fetch_add(1, Ordering::SeqCst);
        self.read_package
            .lock()
            .expect("read package lock")
            .take()
            .ok_or_else(|| "测试导入包缺失".to_string())
    }
    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        _bytes: &[u8],
        _label: &str,
    ) -> Result<(), String> {
        self.write_calls.fetch_add(1, Ordering::SeqCst);
        self.written_paths
            .lock()
            .expect("written paths lock")
            .push(relative_path.to_string());
        Ok(())
    }

    async fn rollback_imported_media_files(&mut self) -> Result<(), String> {
        self.rollback_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn export_application_enforces_owner_before_rows_assembly_and_render() {
    let (context, database) = setup().await;
    let owner_user_id = insert_active_user(database.as_ref(), "owner").await;
    let intruder_user_id = insert_active_user(database.as_ref(), "intruder").await;
    let source = insert_freeform_project(database.as_ref(), owner_user_id).await;
    let host = TestVideoStudioTransferHost::default();

    context
        .export_video_studio_project_validated(
            ExportVideoStudioProjectApplicationInput {
                user_id: intruder_user_id,
                project_id: source.id,
            },
            &host,
        )
        .await
        .expect_err("非 Owner 不得导出视频画布项目包");
    assert_eq!(host.render_calls.load(Ordering::SeqCst), 0);

    let exported = context
        .export_video_studio_project_validated(
            ExportVideoStudioProjectApplicationInput {
                user_id: source.user_id,
                project_id: source.id,
            },
            &host,
        )
        .await
        .expect("视频画布导出 application 应成功");

    assert_eq!(exported.project_title, source.title);
    assert_eq!(exported.package_bytes, vec![1, 2, 3]);
    assert_eq!(host.render_calls.load(Ordering::SeqCst), 1);
    let package = host.rendered_package.lock().expect("rendered package lock");
    let package = package.as_ref().expect("application 应组装导出包");
    assert_eq!(package.project.id, source.id);
    assert_eq!(package.version, VIDEO_STUDIO_PACKAGE_VERSION);
    assert_eq!(package.mode, PACKAGE_MODE);
}

#[tokio::test]
async fn import_application_owns_archive_validation_database_import_and_user_projection() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 8).await;
    let host_package = ReadVideoStudioProjectPackage {
        package: empty_package(database.as_ref(), source.id).await,
        media_payload: ImportMediaPayload {
            entries: HashMap::new(),
            bytes: HashMap::new(),
        },
    };
    let mut host = TestVideoStudioTransferHost::with_read_package(host_package);

    let imported = context
        .import_video_studio_project_validated(
            ImportProjectPackageApplicationInput {
                user_id: 81,
                package_bytes: vec![9],
            },
            &mut host,
        )
        .await
        .expect("视频画布导入 application 应成功");

    let imported_project = a4n_db::project::project_by_id_record(database.as_ref(), imported)
        .await
        .expect("应查询导入项目")
        .expect("导入项目应存在");
    assert_eq!(imported_project.user_id, 81);
    assert_eq!(host.read_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.write_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn import_application_builds_canonical_media_path_before_host_write() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 10).await;
    let bytes = b"video bytes".to_vec();
    let mut package = empty_package(database.as_ref(), source.id).await;
    package.media_assets.push(media_asset::Model {
        id: 10,
        project_id: source.id,
        root_project_id: source.id,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "imported".to_string(),
        file_path: Some("projects/source/video.mp4".to_string()),
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        file_size: Some(bytes.len() as i64),
        checksum: None,
        status: "completed".to_string(),
        error_message: None,
        created_at: source.created_at,
        updated_at: source.updated_at,
    });
    let entry = MediaFileEntry {
        owner_kind: CanvasNodeRefKind::MediaAsset.as_str().to_string(),
        owner_id: 10,
        field: FILE_PATH_FIELD.to_string(),
        original_path: "projects/source/video.mp4".to_string(),
        package_path: "media/media_asset/10/file.mp4".to_string(),
    };
    package.media_files.push(entry.clone());
    let mut entries = HashMap::new();
    entries.insert(
        media_file_key(CanvasNodeRefKind::MediaAsset.as_str(), 10, FILE_PATH_FIELD),
        entry,
    );
    let mut payload_bytes = HashMap::new();
    payload_bytes.insert("media/media_asset/10/file.mp4".to_string(), bytes);
    let mut host = TestVideoStudioTransferHost::with_read_package(ReadVideoStudioProjectPackage {
        package,
        media_payload: ImportMediaPayload {
            entries,
            bytes: payload_bytes,
        },
    });

    let imported = context
        .import_video_studio_project_validated(
            ImportProjectPackageApplicationInput {
                user_id: 101,
                package_bytes: vec![9],
            },
            &mut host,
        )
        .await
        .expect("带媒体的视频画布导入 application 应成功");

    assert_eq!(host.write_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        host.written_paths
            .lock()
            .expect("written paths lock")
            .as_slice(),
        [format!(
            "projects/{}/media/video/imported_package_media_10_file_path.mp4",
            imported
        )]
    );
}

#[tokio::test]
async fn import_application_rejects_invalid_package_before_file_rollback() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 9).await;
    let mut package = empty_package(database.as_ref(), source.id).await;
    package.version = "invalid-version".to_string();
    let mut host = TestVideoStudioTransferHost::with_read_package(ReadVideoStudioProjectPackage {
        package,
        media_payload: ImportMediaPayload {
            entries: HashMap::new(),
            bytes: HashMap::new(),
        },
    });

    let error = context
        .import_video_studio_project_validated(
            ImportProjectPackageApplicationInput {
                user_id: 91,
                package_bytes: vec![9],
            },
            &mut host,
        )
        .await
        .expect_err("非法导入包必须失败");

    assert!(error.to_string().contains("版本无效"));
    assert_eq!(host.read_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn media_package_path_preserves_owner_field_and_extension() {
    let path = media_package_path("generated_video", 42, "thumbnail", "poster.webp")
        .expect("媒体包路径应有效");

    assert_eq!(path, "media/generated_video/42/thumbnail.webp");
}

#[test]
fn media_package_path_rejects_missing_extension() {
    let error = media_package_path("media_asset", 7, "file", "projects/7/video")
        .expect_err("媒体文件扩展名必须存在");

    assert_eq!(error, "媒体文件缺少扩展名: projects/7/video");
}

async fn empty_package(
    database: &a4n_db::DatabaseConnection,
    project_id: i64,
) -> VideoStudioProjectPackage {
    let project = a4n_db::project::project_by_id_record(database, project_id)
        .await
        .expect("项目记录应读取成功")
        .expect("项目应存在");
    VideoStudioProjectPackage {
        version: VIDEO_STUDIO_PACKAGE_VERSION.to_string(),
        exported_at: "2026-07-22 00:00:00".to_string(),
        mode: PACKAGE_MODE.to_string(),
        project,
        canvas_graphs: Vec::new(),
        canvas_nodes: Vec::new(),
        canvas_edges: Vec::new(),
        canvas_text_assets: Vec::new(),
        generation_specs: Vec::new(),
        prompt_artifacts: Vec::new(),
        generated_images: Vec::new(),
        generated_videos: Vec::new(),
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
        media_files: Vec::new(),
    }
}

async fn setup() -> (OrchestratorContext, Arc<a4n_db::DatabaseConnection>) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (context, database)
}

async fn insert_active_user(database: &a4n_db::DatabaseConnection, label: &str) -> i64 {
    let now = chrono::Utc::now().naive_utc();
    user::ActiveModel {
        username: sea_orm::Set(format!("video-studio-transfer-{label}")),
        email: sea_orm::Set(format!("video-studio-transfer-{label}@example.test")),
        password_hash: sea_orm::Set("test-password-hash".to_string()),
        role: sea_orm::Set("user".to_string()),
        oauth_provider: sea_orm::Set(None),
        oauth_subject: sea_orm::Set(None),
        avatar_url: sea_orm::Set(None),
        is_active: sea_orm::Set(true),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        last_login_at: sea_orm::Set(None),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试用户应可写入")
    .id
}

async fn insert_freeform_project(
    database: &a4n_db::DatabaseConnection,
    user_id: i64,
) -> a4n_domain::project::ProjectRecord {
    create_freeform_video_project_record(
        database,
        user_id,
        CreateFreeformVideoProjectParams {
            title: format!("video-studio-transfer-{user_id}"),
            description: "video studio transfer application test".to_string(),
        },
    )
    .await
    .expect("测试 freeform 视频项目应可写入")
}
