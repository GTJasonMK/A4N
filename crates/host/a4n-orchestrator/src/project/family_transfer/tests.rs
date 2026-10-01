use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use a4n_api_contract::project_transfer::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    ImportProjectPackageApplicationInput, ProjectPackageExportInput, ProjectPackageImportPayload,
    ProjectPackageTransferHostPort,
};
use a4n_archive_core::{ArchiveEntry, write_zip};
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter, chapter_manga_prompt, chapter_shooting_script},
    character::{character_portrait, protagonist_profile},
    media::media_asset,
    project::{novel_project, project_source_snapshot, source_snapshot_chapter},
    user::user,
};
use a4n_domain::project::ProjectRecord;
use a4n_domain::shooting_script::{ChapterShootingScriptV2, ShootingVisualPlanSourceV1};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};

use crate::project::{
    FAMILY_PACKAGE_MANIFEST_PATH, ImportMediaPayload, PROJECT_FAMILY_PACKAGE_VERSION,
    ProjectFamilyPackage, hash_json, hash_text, media_file_key, sha256_bytes,
    validate_family_media_manifest, validate_family_package_manifest,
};
use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::{
    MediaFileEntry, ProjectFamilyExportRows, imported_project_family_media_path,
    project_family_export_media_files,
};

struct ReadProjectFamilyPackage {
    package: ProjectFamilyPackage,
    media_payload: ImportMediaPayload,
}

impl ReadProjectFamilyPackage {
    fn into_host_payload(self) -> ProjectPackageImportPayload {
        ProjectPackageImportPayload {
            manifest: serde_json::to_value(self.package).expect("serialize test package"),
            media_bytes: self.media_payload.bytes,
        }
    }
}

#[derive(Default)]
struct TestProjectFamilyTransferHost {
    render_calls: AtomicUsize,
    rendered_mode: Mutex<Option<String>>,
    rendered_project_ids: Mutex<Vec<i64>>,
    read_package: Mutex<Option<ProjectPackageImportPayload>>,
    written_paths: Vec<String>,
    rollback_calls: AtomicUsize,
}

#[derive(Default)]
struct ZipProjectFamilyTransferHost {
    files: Mutex<HashMap<String, Vec<u8>>>,
    written_paths: Vec<String>,
}

impl ZipProjectFamilyTransferHost {
    fn new(files: HashMap<String, Vec<u8>>) -> Self {
        Self {
            files: Mutex::new(files),
            written_paths: Vec::new(),
        }
    }

    fn read_package(package_bytes: &[u8]) -> Result<ReadProjectFamilyPackage, String> {
        let (manifest, bytes) = a4n_import_core::read_media_package_zip_bytes(
            package_bytes,
            FAMILY_PACKAGE_MANIFEST_PATH,
            "项目族 round-trip 包",
            "项目族 round-trip 清单",
        )
        .map_err(|error| error.to_string())?;
        let package: ProjectFamilyPackage =
            a4n_import_core::parse_json_manifest_value(manifest, "项目族 round-trip 清单")
                .map_err(|error| error.to_string())?;
        let mut entries = HashMap::new();
        for entry in &package.media_files {
            let key = media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
            if entries.insert(key, entry.clone()).is_some() {
                return Err(format!(
                    "项目族 round-trip 媒体索引重复: owner_kind={}, owner_id={}, field={}",
                    entry.owner_kind, entry.owner_id, entry.field
                ));
            }
        }
        Ok(ReadProjectFamilyPackage {
            package,
            media_payload: ImportMediaPayload { entries, bytes },
        })
    }
}

impl TestProjectFamilyTransferHost {
    fn with_read_package(read_package: ReadProjectFamilyPackage) -> Self {
        Self {
            read_package: Mutex::new(Some(read_package.into_host_payload())),
            ..Self::default()
        }
    }
}

#[async_trait]
impl ProjectPackageTransferHostPort for TestProjectFamilyTransferHost {
    async fn rollback_imported_project(
        &self,
        _user_id: i64,
        _project_id: i64,
    ) -> Result<(), String> {
        panic!("family import must roll back its transaction");
    }

    async fn render_export_package(
        &self,
        input: ProjectPackageExportInput,
    ) -> Result<Vec<u8>, String> {
        let package: ProjectFamilyPackage =
            serde_json::from_value(input.manifest).map_err(|error| error.to_string())?;
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
        _manifest_path: &str,
        _package_label: &str,
        _manifest_label: &str,
    ) -> Result<ProjectPackageImportPayload, String> {
        self.read_package
            .lock()
            .expect("read package lock")
            .take()
            .ok_or_else(|| "missing test package".to_string())
    }
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

#[async_trait]
impl ProjectPackageTransferHostPort for ZipProjectFamilyTransferHost {
    async fn rollback_imported_project(
        &self,
        _user_id: i64,
        _project_id: i64,
    ) -> Result<(), String> {
        panic!("family import must roll back its transaction");
    }

    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        bytes: &[u8],
        _label: &str,
    ) -> Result<(), String> {
        if self
            .files
            .lock()
            .expect("round-trip files lock")
            .insert(relative_path.to_string(), bytes.to_vec())
            .is_some()
        {
            return Err(format!("round-trip 导入媒体路径已存在: {relative_path}"));
        }
        self.written_paths.push(relative_path.to_string());
        Ok(())
    }

    async fn rollback_imported_media_files(&mut self) -> Result<(), String> {
        let mut files = self.files.lock().expect("round-trip files lock");
        for path in self.written_paths.drain(..) {
            files.remove(&path);
        }
        Ok(())
    }

    async fn render_export_package(
        &self,
        input: ProjectPackageExportInput,
    ) -> Result<Vec<u8>, String> {
        let files = self.files.lock().expect("round-trip files lock");
        let entries = input
            .media_files
            .iter()
            .map(|entry| {
                let bytes = files
                    .get(&entry.original_path)
                    .cloned()
                    .ok_or_else(|| format!("round-trip 导出媒体不存在: {}", entry.original_path))?;
                Ok(ArchiveEntry {
                    path: entry.package_path.clone(),
                    bytes,
                    label: format!("round-trip 媒体 {}", entry.package_path),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let manifest_bytes =
            serde_json::to_vec(&input.manifest).map_err(|error| error.to_string())?;
        write_zip(
            std::iter::once(ArchiveEntry {
                path: input.manifest_path,
                bytes: manifest_bytes,
                label: input.manifest_label,
            })
            .chain(entries),
            input.package_label,
        )
        .map_err(|error| error.to_string())
    }

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
        manifest_path: &str,
        package_label: &str,
        manifest_label: &str,
    ) -> Result<ProjectPackageImportPayload, String> {
        let (manifest, media_bytes) = a4n_import_core::read_media_package_zip_bytes(
            &package_bytes,
            manifest_path,
            package_label,
            manifest_label,
        )
        .map_err(|error| error.to_string())?;
        Ok(ProjectPackageImportPayload {
            manifest,
            media_bytes,
        })
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

    assert_eq!(exported.project_title, root.title);
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

    assert_eq!(exported.project_title, derived.title);
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
    rows.generated_videos.push(generated_video::Model {
        id: 11,
        project_id: 2,
        chapter_id: None,
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
        workflow_mode: None,
        file_path: Some("projects/2/video/output.webm".to_string()),
        thumbnail_path: Some("projects/2/video/output.jpg".to_string()),
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "test".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 5.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string(),
        provider_job_id: None,
        error_message: None,
        timeline_item_id: None,
        metadata_json: None,
        is_favorite: false,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    });

    let media_files = project_family_export_media_files(&rows).expect("媒体索引应可组装");
    assert_eq!(media_files.len(), 4);
    assert!(
        media_files
            .iter()
            .any(|entry| entry.package_path == "media/media_asset/10/file_path.mp4")
    );
    assert!(
        media_files
            .iter()
            .any(|entry| entry.package_path == "media/media_asset/10/thumbnail_path.png")
    );
    assert!(
        media_files
            .iter()
            .any(|entry| entry.package_path == "media/generated_video/11/file_path.webm")
    );
    assert!(
        media_files
            .iter()
            .any(|entry| entry.package_path == "media/generated_video/11/thumbnail_path.jpg")
    );
    assert_eq!(
        imported_project_family_media_path(
            20,
            "media_asset",
            30,
            a4n_db_models::models::media::media_asset::THUMBNAIL_PATH_FIELD,
            "media/x.png",
        )
        .expect("导入路径应可生成"),
        "projects/20/media/media_asset/imported_package_30_thumbnail_path.png"
    );
}

#[tokio::test]
async fn manga_family_application_round_trip_remaps_all_owned_identities_and_media_paths() {
    let (context, database) = setup().await;
    let owner_user_id = insert_active_user(database.as_ref(), "manga-round-trip").await;
    let (root, source_files) = seed_manga_family_round_trip(database.as_ref(), owner_user_id).await;
    let mut host = ZipProjectFamilyTransferHost::new(source_files);

    let exported = context
        .export_project_family_package_validated(
            ExportProjectFamilyPackageApplicationInput {
                user_id: owner_user_id,
                root_project_id: root.id,
                include_derived_projects: true,
            },
            &host,
        )
        .await
        .expect("完整漫画项目族应可导出真实 ZIP");
    let original = ZipProjectFamilyTransferHost::read_package(&exported.package_bytes)
        .expect("真实 ZIP 应可由 import-core 读取");
    validate_family_package_manifest(&original.package).expect("原始项目族清单应通过严格校验");
    validate_family_media_manifest(&original.package, &original.media_payload)
        .expect("原始项目族媒体应通过严格校验");

    let imported = context
        .import_project_family_package_validated(
            ImportProjectPackageApplicationInput {
                user_id: owner_user_id,
                package_bytes: exported.package_bytes,
            },
            &mut host,
        )
        .await
        .expect("完整漫画项目族应通过 application 导入");
    assert_eq!(imported.len(), 3);
    let imported_root = imported
        .iter()
        .find(|project| project.project_kind == "text")
        .expect("导入结果应包含根文本项目");
    assert_ne!(imported_root.id, original.package.root_project_id);

    let reexported = context
        .export_project_family_package_validated(
            ExportProjectFamilyPackageApplicationInput {
                user_id: owner_user_id,
                root_project_id: imported_root.id,
                include_derived_projects: true,
            },
            &host,
        )
        .await
        .expect("导入后的漫画项目族应可再次导出真实 ZIP");
    let remapped = ZipProjectFamilyTransferHost::read_package(&reexported.package_bytes)
        .expect("再次导出的真实 ZIP 应可读取");
    validate_family_package_manifest(&remapped.package).expect("重映射清单应通过严格校验");
    validate_family_media_manifest(&remapped.package, &remapped.media_payload)
        .expect("重映射媒体应通过严格校验");

    let old_root = original
        .package
        .projects
        .iter()
        .find(|project| project.project_kind == "text")
        .expect("原始清单应包含根项目");
    let old_comic = original
        .package
        .projects
        .iter()
        .find(|project| project.project_kind == "comic")
        .expect("原始清单应包含漫画派生项目");
    let new_root = remapped
        .package
        .projects
        .iter()
        .find(|project| project.project_kind == "text")
        .expect("重映射清单应包含根项目");
    let new_comic = remapped
        .package
        .projects
        .iter()
        .find(|project| project.project_kind == "comic")
        .expect("重映射清单应包含漫画派生项目");
    assert_ne!(old_root.id, new_root.id);
    assert_ne!(old_comic.id, new_comic.id);
    assert_eq!(remapped.package.root_project_id, new_root.id);
    assert_eq!(new_comic.root_project_id, Some(new_root.id));

    let old_chapter = &original.package.text_core.chapters[0];
    let new_chapter = &remapped.package.text_core.chapters[0];
    let old_profile = &original.package.text_core.character_profiles[0];
    let new_profile = &remapped.package.text_core.character_profiles[0];
    let old_snapshot = &original.package.source_snapshots[0];
    let new_snapshot = &remapped.package.source_snapshots[0];
    let old_snapshot_chapter = &original.package.source_snapshot_chapters[0];
    let new_snapshot_chapter = &remapped.package.source_snapshot_chapters[0];
    assert_ne!(old_chapter.id, new_chapter.id);
    assert_ne!(old_profile.id, new_profile.id);
    assert_ne!(old_snapshot.id, new_snapshot.id);
    assert_ne!(old_snapshot_chapter.id, new_snapshot_chapter.id);
    assert_eq!(new_chapter.project_id, new_root.id);
    assert_eq!(new_profile.project_id, new_root.id);
    assert_eq!(new_snapshot.root_project_id, new_root.id);
    assert_eq!(new_snapshot_chapter.snapshot_id, new_snapshot.id);
    assert_eq!(new_snapshot_chapter.root_project_id, new_root.id);
    assert_eq!(new_snapshot_chapter.source_chapter_id, new_chapter.id);
    assert_eq!(
        new_snapshot.chapter_hashes_json[0]["source_chapter_id"],
        json!(new_chapter.id)
    );
    assert_eq!(new_snapshot.profile_hashes_json, json!([]));
    assert_eq!(new_comic.source_snapshot_id, Some(new_snapshot.id));

    let old_plan = &original.package.chapter_visual_plans[0];
    let new_plan = &remapped.package.chapter_visual_plans[0];
    assert_ne!(old_plan.id, new_plan.id);
    assert_ne!(old_plan.plan_sha256, new_plan.plan_sha256);
    let mut expected_plan: a4n_domain::chapter_visual_plan::ChapterVisualPlanV3 =
        serde_json::from_value(old_plan.plan_json.clone()).unwrap();
    expected_plan
        .remap_resources(
            |id| {
                original
                    .package
                    .generated_images
                    .iter()
                    .position(|row| row.id == id)
                    .map(|index| remapped.package.generated_images[index].id)
                    .ok_or_else(|| "missing image".into())
            },
            |id| {
                original
                    .package
                    .text_core
                    .scene_profiles
                    .iter()
                    .position(|row| row.id == id)
                    .map(|index| remapped.package.text_core.scene_profiles[index].id)
                    .ok_or_else(|| "missing scene".into())
            },
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(expected_plan).unwrap(),
        new_plan.plan_json
    );
    assert_eq!(new_plan.root_project_id, new_root.id);
    assert_eq!(new_plan.source_chapter_id, new_chapter.id);
    assert_eq!(new_plan.source_snapshot_id, Some(new_snapshot.id));
    assert_eq!(
        new_plan.source_snapshot_chapter_id,
        Some(new_snapshot_chapter.id)
    );
    assert_eq!(new_plan.author_user_id, owner_user_id);
    assert_eq!(old_snapshot_chapter.content, new_snapshot_chapter.content);
    for source in [old_snapshot_chapter, new_snapshot_chapter] {
        let value = serde_json::to_value(source).unwrap();
        assert!(value.get("visual_plan_id").is_none());
    }
    let movie = remapped
        .package
        .projects
        .iter()
        .find(|project| project.project_kind == "video")
        .unwrap();
    assert_eq!(new_plan.project_id, movie.id);

    let old_script = &original.package.chapter_shooting_scripts[0];
    let new_script = &remapped.package.chapter_shooting_scripts[0];
    assert_ne!(old_script.id, new_script.id);
    assert_ne!(old_script.script_sha256, new_script.script_sha256);
    assert_eq!(new_script.root_project_id, new_root.id);
    assert_eq!(new_script.source_chapter_id, new_chapter.id);
    assert_eq!(new_script.source_snapshot_id, Some(new_snapshot.id));
    assert_eq!(
        new_script.source_snapshot_chapter_id,
        Some(new_snapshot_chapter.id)
    );
    let new_source_revision_key = format!(
        "snapshot:{}:chapter:{}:{}",
        new_snapshot.id, new_snapshot_chapter.id, new_script.source_content_sha256
    );
    assert_eq!(new_script.source_revision_key, new_source_revision_key);
    assert_eq!(old_script.visual_plan_id, Some(old_plan.id));
    assert_eq!(new_script.visual_plan_id, Some(new_plan.id));
    assert_eq!(new_script.visual_plan_version, Some(new_plan.plan_version));
    assert_eq!(
        new_script.visual_plan_sha256.as_deref(),
        Some(new_plan.plan_sha256.as_str())
    );
    let typed_new_script: ChapterShootingScriptV2 =
        serde_json::from_value(new_script.script_json.clone())
            .expect("重映射拍摄剧本 JSON 应保持合法");
    let typed_plan_source = typed_new_script
        .visual_plan_source
        .expect("重映射拍摄剧本应保留 visual plan 来源");
    assert_eq!(typed_plan_source.plan_id, new_plan.id);
    assert_eq!(typed_plan_source.plan_version, new_plan.plan_version);
    assert_eq!(typed_plan_source.plan_sha256, new_plan.plan_sha256);

    let old_portrait = &original.package.character_portraits[0];
    let new_portrait = &remapped.package.character_portraits[0];
    assert_ne!(old_portrait.id, new_portrait.id);
    assert_eq!(new_portrait.project_id, new_root.id);
    assert_eq!(new_portrait.profile_id, new_profile.id);
    assert_eq!(
        new_portrait.image_path,
        format!(
            "projects/{}/media/character_portrait/imported_package_{}_image_path.png",
            new_root.id, new_portrait.id
        )
    );

    let old_reference_image = original
        .package
        .generated_images
        .iter()
        .find(|image| image.panel_id.as_deref() == Some("reference"))
        .expect("原始清单应包含参考图片");
    let old_result_image = original
        .package
        .generated_images
        .iter()
        .find(|image| image.panel_id.as_deref() == Some("page-1"))
        .expect("原始清单应包含结果图片");
    let new_reference_image = remapped
        .package
        .generated_images
        .iter()
        .find(|image| image.panel_id.as_deref() == Some("reference"))
        .expect("重映射清单应包含参考图片");
    let new_result_image = remapped
        .package
        .generated_images
        .iter()
        .find(|image| image.panel_id.as_deref() == Some("page-1"))
        .expect("重映射清单应包含结果图片");
    assert_ne!(old_reference_image.id, new_reference_image.id);
    assert_ne!(old_result_image.id, new_result_image.id);
    for image in [new_reference_image, new_result_image] {
        assert_eq!(image.project_id, new_comic.id);
        assert_eq!(image.chapter_id, Some(new_chapter.id));
        assert_eq!(
            image.file_path,
            format!(
                "projects/{}/media/generated_image/imported_package_{}_file_path.png",
                new_comic.id, image.id
            )
        );
    }

    let old_prompt = &original.package.chapter_manga_prompts[0];
    let new_prompt = &remapped.package.chapter_manga_prompts[0];
    assert_ne!(old_prompt.id, new_prompt.id);
    assert_eq!(new_prompt.project_id, new_comic.id);
    assert_eq!(new_prompt.chapter_id, new_chapter.id);
    assert_eq!(new_prompt.generated_image_id, Some(new_result_image.id));
    let new_layout = new_prompt
        .layout
        .as_ref()
        .expect("漫画提示词应保留本地计划布局");
    assert_eq!(
        new_layout["schema_version"],
        json!("manga_page_generation_v3")
    );
    assert!(new_layout.get("shooting_script_id").is_none());
    assert!(new_layout.get("shooting_script_revision").is_none());
    assert_eq!(new_layout["source"]["root_project_id"], json!(new_root.id));
    assert_eq!(
        new_layout["source"]["source_chapter_id"],
        json!(new_chapter.id)
    );
    assert_eq!(
        new_layout["source"]["source_snapshot_id"],
        json!(new_snapshot.id)
    );
    assert_eq!(
        new_layout["source"]["source_snapshot_chapter_id"],
        json!(new_snapshot_chapter.id)
    );
    assert_eq!(
        new_layout["source"]["source_revision_key"],
        json!(new_source_revision_key)
    );
    assert!(new_layout["source"].get("shooting_script_id").is_none());
    let adaptation = remapped
        .package
        .authoring_documents
        .iter()
        .find(|row| row.kind == "adaptation_plan")
        .unwrap();
    assert_eq!(new_layout["source"]["adaptation_plan_id"], adaptation.id);
    assert_eq!(adaptation.project_id, new_comic.id);
    assert_eq!(
        new_layout["source"]["adaptation_plan_sha256"],
        adaptation.document_sha256
    );
    assert_eq!(
        new_layout["reference_bindings"][0]["bound_asset"]["asset_id"],
        json!(new_portrait.id)
    );
    assert_eq!(
        new_layout["reference_bindings"][0]["bound_asset"]["image_path"],
        json!(new_portrait.image_path)
    );
    assert_eq!(
        new_layout["reference_bindings"][1]["bound_asset"]["asset_id"],
        json!(new_reference_image.id)
    );
    assert_eq!(
        new_layout["reference_bindings"][1]["bound_asset"]["image_path"],
        json!(new_reference_image.file_path)
    );

    let result_metadata = new_result_image
        .metadata_json
        .as_ref()
        .expect("漫画结果图片应保留 metadata_json");
    assert_eq!(result_metadata["page_prompt_id"], json!(new_prompt.id));
    assert_eq!(
        result_metadata["reference_bindings"][0]["bound_asset"]["asset_id"],
        json!(new_portrait.id)
    );
    assert_eq!(
        result_metadata["reference_bindings"][1]["bound_asset"]["asset_id"],
        json!(new_reference_image.id)
    );
    assert_eq!(
        result_metadata["generation"]["chapter_id"],
        json!(new_chapter.id)
    );
    assert_eq!(
        result_metadata["generation_context"]["reference_bindings"][1]["bound_asset"]["image_path"],
        json!(new_reference_image.file_path)
    );

    for (owner_kind, owner_id, field, original_path) in [
        (
            "character_portrait",
            new_portrait.id,
            "image_path",
            new_portrait.image_path.as_str(),
        ),
        (
            "generated_image",
            new_reference_image.id,
            "file_path",
            new_reference_image.file_path.as_str(),
        ),
        (
            "generated_image",
            new_result_image.id,
            "file_path",
            new_result_image.file_path.as_str(),
        ),
    ] {
        let entry = remapped
            .package
            .media_files
            .iter()
            .find(|entry| {
                entry.owner_kind == owner_kind && entry.owner_id == owner_id && entry.field == field
            })
            .expect("重映射 owner 应拥有唯一媒体条目");
        assert_eq!(entry.original_path, original_path);
        assert_eq!(
            entry.package_path,
            format!("media/{owner_kind}/{owner_id}/{field}.png")
        );
        assert!(
            remapped
                .media_payload
                .bytes
                .contains_key(&entry.package_path)
        );
    }
    assert_eq!(host.written_paths.len(), remapped.package.media_files.len());
}

async fn seed_manga_family_round_trip(
    database: &a4n_db::DatabaseConnection,
    user_id: i64,
) -> (novel_project::Model, HashMap<String, Vec<u8>>) {
    let now = Utc::now().naive_utc();
    let root = insert_project(database, user_id, "text", None).await;
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("主角推开大厅门并停在暗廊前。".to_string()),
        word_count: Set(16),
        status: Set("completed".to_string()),
        summary: Set(Some("主角发现暗廊异常".to_string())),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 根章节应可写入");
    let profile = protagonist_profile::ActiveModel {
        project_id: Set(root.id),
        name: Set("主角".to_string()),
        gender: Set(None),
        age: Set(None),
        base_attributes: Set(json!({})),
        current_state: Set(json!({})),
        character_arc: Set(None),
        voice_fingerprint: Set(None),
        appearance: Set(Some("黑发，深色外套".to_string())),
        background: Set(None),
        personality: Set(Some("谨慎".to_string())),
        last_synced_chapter: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 角色档案应可写入");
    let content_hash = hash_text(&chapter.content);
    let chapter_hashes = json!([{
        "source_chapter_id": chapter.id,
        "chapter_number": chapter.chapter_number,
        "content_hash": content_hash,
        "analysis_hash": null
    }]);
    let chapter_scope = json!({"start": 1, "end": 1});
    let profile_hashes = json!([]);
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root.id),
        source_version_hash: Set(hash_json(&json!({
            "root_project_id": root.id,
            "chapter_scope": chapter_scope,
            "chapter_hashes": chapter_hashes,
            "profile_hashes": profile_hashes,
            "blueprint_version_id": null
        }))),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(chapter_scope),
        chapter_hashes_json: Set(chapter_hashes),
        profile_hashes_json: Set(profile_hashes),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 源快照应可写入");
    let snapshot_chapter = source_snapshot_chapter::ActiveModel {
        snapshot_id: Set(snapshot.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(chapter.id),
        chapter_number: Set(chapter.chapter_number),
        title: Set(chapter.title.clone()),
        content: Set(chapter.content.clone()),
        summary: Set(chapter.summary.clone()),
        analysis_data_json: Set(None),
        content_hash: Set(content_hash.clone()),
        analysis_hash: Set(None),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 源快照章节应可写入");
    let comic = novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set("round-trip-comic".to_string()),
        description: Set("round-trip comic derived project".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("comic".to_string()),
        root_project_id: Set(Some(root.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_sync_status: Set("current".to_string()),
        status: Set("draft".to_string()),
        target_chapters: Set(1),
        current_chapter: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 漫画派生项目应可写入");
    let mut video: novel_project::ActiveModel = comic.clone().into();
    video.id = sea_orm::NotSet;
    video.project_kind = Set("video".into());
    video.title = Set("round-trip-video".into());
    let video = video.insert(database).await.unwrap();

    let source = a4n_domain::chapter::ResolvedChapterSource {
        requested_project_id: video.id,
        root_project_id: root.id,
        source_chapter_id: chapter.id,
        source_snapshot_id: Some(snapshot.id),
        source_snapshot_chapter_id: Some(snapshot_chapter.id),
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        summary: chapter.summary.clone(),
        analysis_data_json: None,
        content: chapter.content.clone(),
        content_sha256: content_hash.clone(),
        source_revision_key: format!(
            "snapshot:{}:chapter:{}:{}",
            snapshot.id, snapshot_chapter.id, content_hash
        ),
    };
    let source_spans = a4n_domain::source_spans::source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        &chapter.content,
    );
    let mut plan = crate::shooting_script::visual_plan_fixture(&source).plan;
    let scene_id =
        crate::shooting_script::seed_visual_plan_scene(database, source.requested_project_id).await;
    plan.points.remove(1);
    plan.points[1].at_ms = 2000;
    plan.segments.truncate(1);
    plan.segments[0].end_point_id = "end".into();
    plan.segments[0].scene_binding = Some(a4n_domain::chapter_visual_plan::SceneBindingV3::Scene {
        scene_profile_id: scene_id,
    });
    let draft = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
        database,
        a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
            source,
            author_user_id: user_id,
            authoring_mode: a4n_domain::chapter_visual_plan::ChapterVisualPlanAuthoringMode::Manual,
            expected: None,
            expected_source_hash: None,
            plan,
        },
    )
    .await
    .expect("round-trip visual plan 草稿应可写入");
    let visual_plan = a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
        database,
        a4n_domain::chapter_visual_plan::ConfirmChapterVisualPlanInput {
            record_id: draft.record.id,
            expected_revision: draft.record.revision,
        },
    )
    .await
    .expect("round-trip visual plan 应可确认")
    .visual_plan;

    let typed_script = round_trip_shooting_script(&visual_plan, &source_spans);
    typed_script
        .validate()
        .expect("round-trip 拍摄剧本 fixture 应满足领域合同");
    let script_json = serde_json::to_value(&typed_script).expect("序列化拍摄剧本 fixture");
    let script_sha256 =
        sha256_bytes(&serde_json::to_vec(&typed_script).expect("序列化拍摄剧本 fixture 字节"));
    let source_revision_key = format!(
        "snapshot:{}:chapter:{}:{}",
        snapshot.id, snapshot_chapter.id, content_hash
    );
    let script = chapter_shooting_script::ActiveModel {
        project_id: Set(video.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(chapter.id),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        source_revision_key: Set(source_revision_key.clone()),
        source_content_sha256: Set(content_hash.clone()),
        visual_plan_id: Set(Some(visual_plan.record.id)),
        visual_plan_version: Set(Some(visual_plan.record.plan_version)),
        visual_plan_sha256: Set(Some(visual_plan.record.plan_sha256.clone())),
        input_fingerprint_sha256: Set(sha256_bytes(b"round-trip shooting input")),
        contract_version: Set("chapter_shooting_script_v2".to_string()),
        revision: Set(1),
        script_sha256: Set(script_sha256.clone()),
        script_json: Set(script_json),
        workflow_run_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 拍摄剧本应可写入");

    let portrait_path = format!("projects/{}/portraits/hero.png", root.id);
    let portrait = character_portrait::ActiveModel {
        profile_id: Set(profile.id),
        project_id: Set(root.id),
        portrait_type: Set("full_body".to_string()),
        image_path: Set(portrait_path.clone()),
        prompt: Set("主角全身立绘".to_string()),
        expression: Set(Some("警惕".to_string())),
        pose: Set(Some("站立".to_string())),
        is_primary: Set(true),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 角色立绘应可写入");
    let reference_image_path = format!("projects/{}/images/reference.png", comic.id);
    let reference_image_bytes = b"round-trip-reference-image".to_vec();
    let reference_image = generated_image::ActiveModel {
        project_id: Set(comic.id),
        image_type: Set("illustration".to_string()),
        file_path: Set(reference_image_path.clone()),
        prompt: Set("主角参考图".to_string()),
        model: Set("round-trip-model".to_string()),
        width: Set(512),
        height: Set(512),
        file_size: Set(Some(reference_image_bytes.len() as i64)),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter.id)),
        panel_id: Set(Some("reference".to_string())),
        metadata_json: Set(None),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 参考生成图片应可写入");
    let result_image_path = format!("projects/{}/images/page-1.png", comic.id);
    let result_image_bytes = b"round-trip-result-image".to_vec();
    let result_image = generated_image::ActiveModel {
        project_id: Set(comic.id),
        image_type: Set("manga_page".to_string()),
        file_path: Set(result_image_path.clone()),
        prompt: Set("漫画第一页".to_string()),
        model: Set("round-trip-model".to_string()),
        width: Set(1024),
        height: Set(1536),
        file_size: Set(Some(result_image_bytes.len() as i64)),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter.id)),
        panel_id: Set(Some("page-1".to_string())),
        metadata_json: Set(None),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 漫画结果图片应可写入");
    let reference_bindings = json!([
        {
            "character_name": "主角",
            "bound_asset": {
                "asset_kind": "character_portrait",
                "asset_id": portrait.id,
                "image_path": portrait.image_path
            }
        },
        {
            "character_name": "主角参考图",
            "bound_asset": {
                "asset_kind": "generated_image",
                "asset_id": reference_image.id,
                "image_path": reference_image.file_path
            }
        }
    ]);
    let layout = round_trip_manga_layout(
        &root,
        &chapter,
        &snapshot,
        &snapshot_chapter,
        &script,
        reference_bindings.clone(),
    );
    let prompt = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter.id),
        project_id: Set(comic.id),
        page_number: Set(1),
        prompt_type: Set("page_generation".to_string()),
        image_prompt: Set("整页漫画：主角停在暗廊前".to_string()),
        dialogue_text: Set(None),
        characters_in_frame: Set(Some(json!(["主角"]))),
        layout: Set(Some(layout.clone())),
        generated_image_id: Set(Some(result_image.id)),
        status: Set("completed".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("round-trip 漫画提示词应可写入");
    generated_image::ActiveModel {
        id: Set(result_image.id),
        metadata_json: Set(Some(json!({
            "page_prompt_id": prompt.id,
            "reference_bindings": reference_bindings,
            "generation": {"chapter_id": chapter.id},
            "generation_context": {
                "reference_bindings": layout["reference_bindings"].clone()
            }
        }))),
        ..Default::default()
    }
    .update(database)
    .await
    .expect("round-trip 漫画结果图片 metadata 应可更新");
    a4n_migration::migrations::media_authoring::migrate_legacy_media(database)
        .await
        .expect("旧漫画页面应转换为本地计划后再验证包往返");

    (
        root,
        HashMap::from([
            (portrait_path, b"round-trip-portrait".to_vec()),
            (reference_image_path, reference_image_bytes),
            (result_image_path, result_image_bytes),
        ]),
    )
}

fn round_trip_shooting_script(
    visual_plan: &a4n_db::chapter_visual_plan::ParsedChapterVisualPlan,
    source_spans: &[a4n_domain::source_spans::SourceSpan],
) -> ChapterShootingScriptV2 {
    let mut script: ChapterShootingScriptV2 = serde_json::from_str(
        r#"{
        "schema_version": "chapter_shooting_script_v2",
        "directing_statement": {
            "central_dramatic_question": "主角能否确认异常",
            "protagonist_throughline": "克制恐惧并继续调查",
            "audience_experience": "逐步感到紧张",
            "viewpoint_strategy": "限制在主角感知范围",
            "visual_strategy": "保持空间关系清晰",
            "spatial_strategy": "由门口移动到暗廊",
            "sound_strategy": "用脚步和停顿制造悬念",
            "rhythm_strategy": "进入、停顿、观察"
        },
        "scenes": [{
            "scene_index": 1,
            "scene_identity": "hall",
            "sequence_index": 1,
            "narrative_unit_index": 1,
            "timeline_position": 1,
            "intercut_group_id": null,
            "editing_mode": "continuity",
            "transition_contracts": [],
            "title": "进入大厅",
            "slugline": "内景 大厅 夜",
            "location": "大厅",
            "time_of_day": "夜",
            "participants": ["主角"],
            "focal_character": "主角",
            "source_span_ids": ["span-1"],
            "dramatic_question": "暗处是否存在威胁",
            "scene_objective": "确认大厅状况",
            "scene_obstacle": "昏暗遮蔽视线",
            "relationship_before": "主角独自行动",
            "relationship_after": "主角意识到自己可能被观察",
            "audience_knowledge_before": ["大厅昏暗"],
            "audience_knowledge_after": ["暗廊可能存在异常"],
            "withheld_information": ["暗处人物身份"],
            "viewpoint": {
                "mode": "limited_third_person",
                "anchor_character": "主角",
                "information_strategy": "只展示主角可感知的信息"
            },
            "spatial_continuity": {
                "continuity_intent": "transparent",
                "primary_axis": "门口到暗廊",
                "screen_direction": "由左向右",
                "reorientation_strategy": null
            },
            "sound_strategy": "突出脚步和停顿",
            "entry_state": "主角站在门口",
            "exit_state": "主角停在暗廊前",
            "shots": [{
                "shot_index": 1,
                "shot_id": "shot-1",
                "title": "进入",
                "source_span_ids": ["span-1"],
                "coverage": "must_show",
                "narrative_purpose": "建立空间并发现异常",
                "subject": "主角",
                "shot_size": "medium",
                "camera_angle": "eye_level",
                "camera_height": "eye",
                "lens_intent": "保持空间关系清晰",
                "camera_movement": "push_in",
                "composition": "主角位于画面左侧",
                "spatial_anchors": ["门口", "暗廊"],
                "blocking": {"start": "门口", "movement": "走向暗廊", "end": "暗廊前"},
                "performance_beats": [{
                    "beat_index": 1,
                    "beat_id": "beat-1",
                    "source_span_ids": ["span-1"],
                    "actor": "主角",
                    "objective": "确认异常",
                    "obstacle": "视线受阻",
                    "visible_action": "放慢脚步观察暗廊",
                    "reaction": null,
                    "expression": "紧张",
                    "gaze": "看向暗廊",
                    "pose_or_movement": "收紧肩膀",
                    "dialogue_intent": null,
                    "audience_question": "暗处是否有人",
                    "answer_or_delay": "暂不揭示",
                    "knowledge_delta": ["暗廊存在异常"],
                    "relationship_delta": null,
                    "spatial_change": "从门口移动到暗廊",
                    "entry_state": "谨慎",
                    "exit_state": "警惕",
                    "required": true
                }],
                "dialogue_cues": [],
                "sound": {
                    "dialogue": [],
                    "environment": ["大厅回声"],
                    "effects": ["脚步声"],
                    "music": [],
                    "silence": ["观察前停顿"]
                },
                "camera_start_state": "平视中景",
                "camera_end_state": "平视中景",
                "start_state": "主角在门口",
                "end_state": "主角在暗廊前",
                "transition_in": "从开门动作开始",
                "transition_out": "停在暗廊前",
                "continuity_requirements": ["暗廊保持在画面右侧"]
            }]
        }]
    }"#,
    )
    .expect("round-trip 拍摄剧本 JSON fixture 应匹配领域合同");
    let projected_plan = crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
        &visual_plan.plan,
        source_spans,
    )
    .expect("round-trip visual plan 应可投影");
    let unit = &projected_plan.narrative_units[0];
    let planned_shot = &unit.shots[0];
    script.visual_plan_source = Some(ShootingVisualPlanSourceV1 {
        plan_id: visual_plan.record.id,
        plan_version: visual_plan.record.plan_version,
        plan_sha256: visual_plan.record.plan_sha256.clone(),
    });
    script.user_additions = projected_plan.user_additions.clone();
    let scene = &mut script.scenes[0];
    scene.source_span_ids = unit.source_span_ids.clone();
    scene.visual_narrative_unit_id = Some(unit.id.clone());
    scene.user_addition_ids = vec![format!("director:{}", planned_shot.segment_id)];
    let shot = &mut scene.shots[0];
    shot.shot_id = planned_shot.segment_id.clone();
    shot.source_span_ids = unit.source_span_ids.clone();
    shot.visual_shot_id = Some(planned_shot.segment_id.clone());
    shot.planned_start_ms = Some(planned_shot.start_ms);
    shot.planned_end_ms = Some(planned_shot.end_ms);
    shot.user_addition_ids = vec![format!("director:{}", planned_shot.segment_id)];
    shot.visual_constraints = Some(planned_shot.clone());

    shot.performance_beats[0].source_span_ids = unit.source_span_ids.clone();
    shot.performance_beats[0].user_addition_ids =
        vec![format!("director:{}", planned_shot.segment_id)];
    script
}

fn round_trip_manga_layout(
    root: &novel_project::Model,
    chapter: &chapter::Model,
    snapshot: &project_source_snapshot::Model,
    snapshot_chapter: &source_snapshot_chapter::Model,
    script: &chapter_shooting_script::Model,
    reference_bindings: Value,
) -> Value {
    json!({
        "schema_version": "manga_page_generation_v2",
        "page_plan": {
            "page_number": 1,
            "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
            "role": "setup",
            "pacing": "medium",
            "importance": "normal",
            "reading_flow": "top_to_bottom_left_to_right",
            "page_goal": "推进场景",
            "visual_thesis": "主角与暗廊对峙",
            "entry_state": "主角站在门口",
            "exit_state": "主角停在暗廊前",
            "dominant_panel_number": 1,
            "panels": [{
                "panel_number": 1,
                "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
                "function": "dialogue",
                "size": "dominant",
                "shot_scale": "medium",
                "camera_angle": "eye_level",
                "participants": ["主角"],
                "composition": "主角位于左侧，暗廊位于右侧",
                "observable_action": "主角抬头看向暗廊",
                "reaction": "主角收紧肩膀",
                "spatial_relationship": "主角在门口，暗廊在前方",
                "continuity_from_previous": null,
                "text_layers": []
            }]
        },
        "drawing_prompt": {
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": 1,
            "image_prompt": "整页漫画：主角停在暗廊前"
        },
        "reference_bindings": reference_bindings,
        "prompt_fingerprint": {"hash": "round-trip"},
        "source": {
            "root_project_id": root.id,
            "source_chapter_id": chapter.id,
            "source_snapshot_id": snapshot.id,
            "source_snapshot_chapter_id": snapshot_chapter.id,
            "source_revision_key": script.source_revision_key,
            "content_sha256": script.source_content_sha256,
            "shooting_script_id": script.id,
            "shooting_script_revision": 1,
            "shooting_script_sha256": script.script_sha256
        }
    })
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
                    "start": 1,
                    "end": 1
                })),
                chapter_hashes_json: Set(serde_json::json!([])),
                profile_hashes_json: Set(serde_json::json!([])),
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
) -> ProjectRecord {
    let now = Utc::now().naive_utc();
    ProjectRecord {
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

fn empty_package(root: ProjectRecord) -> ProjectFamilyPackage {
    ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: "project_family".to_string(),
        root_project_id: root.id,
        projects: vec![root],
        text_core: a4n_db::project::ProjectFamilyTextCoreRows::default(),
        source_snapshots: Vec::new(),
        source_snapshot_chapters: Vec::new(),
        source_snapshot_outlines: Vec::new(),
        source_snapshot_profiles: Vec::new(),
        text_anchors: Vec::new(),
        profile_evidence: Vec::new(),
        chapter_visual_plans: Vec::new(),
        chapter_shooting_scripts: Vec::new(),
        authoring_documents: Vec::new(),
        authoring_receipts: Vec::new(),
        character_portraits: Vec::new(),
        generated_images: Vec::new(),
        chapter_manga_prompts: Vec::new(),
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
        chapter_video_prompts: Vec::new(),
        generated_videos: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
        source_sync_runs: Vec::new(),
        source_sync_conflicts: Vec::new(),
        source_sync_decisions: Vec::new(),
        media_files: Vec::<MediaFileEntry>::new(),
    }
}

fn empty_media_payload() -> ImportMediaPayload {
    ImportMediaPayload {
        entries: Default::default(),
        bytes: Default::default(),
    }
}
