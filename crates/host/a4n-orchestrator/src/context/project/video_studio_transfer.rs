use a4n_api_contract::project::ProjectSummary;
use a4n_db::canvas::{VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter};
use a4n_error::AppError;
use chrono::Utc;

use super::super::OrchestratorContext;
use crate::project::{
    CanvasProjectTransferApplicationPort, ExportCanvasProjectApplicationInput,
    ExportedProjectPackage, ImportMediaPayload, ImportProjectPackageApplicationInput,
    ProjectPackageTransferError, ProjectPackageTransferHostPort,
    assemble_video_studio_export_package, import_result_with_file_rollback,
    index_import_media_payload, media_file_key, project_package_export_input,
    required_package_extension, validate_canvas_package_manifest, video_studio_export_media_files,
};
use a4n_domain::collaboration::ProjectAccessRole;

impl OrchestratorContext {
    pub(crate) async fn export_canvas_project_validated<Host>(
        &self,
        input: ExportCanvasProjectApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Owner)
            .await?;
        let project = self
            .project_read
            .by_id(input.project_id)
            .await?
            .ok_or_else(|| {
                ProjectPackageTransferError::Message(format!("项目不存在: {}", input.project_id))
            })?;
        let rows = self
            .canvas
            .video_studio_project_export_rows(project.id)
            .await?;
        let media_files = video_studio_export_media_files(&rows)?;
        let mut package = assemble_video_studio_export_package(project.clone(), rows, media_files)?;
        package.provider_slots = self
            .canvas_provider_slots(std::slice::from_ref(&package))
            .await?;
        let host_input = project_package_export_input(
            &package,
            &package.media_files,
            crate::project::MANIFEST_PATH,
            "Canvas 项目清单",
            "Canvas 项目导出包",
        )?;
        let package_bytes = host_port
            .render_export_package(host_input)
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        Ok(ExportedProjectPackage {
            project_title: project.title,
            package_bytes,
        })
    }

    pub(crate) async fn import_canvas_project_validated<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<i64, ProjectPackageTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        if input.package_bytes.is_empty() {
            return Err(ProjectPackageTransferError::Message(
                "Canvas 项目包不能为空".to_string(),
            ));
        }
        let read_package = host_port
            .read_import_package(
                input.package_bytes,
                crate::project::MANIFEST_PATH,
                "Canvas 项目包",
                "Canvas 项目清单",
            )
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        let mut package: crate::project::VideoStudioProjectPackage =
            a4n_import_contract::parse_json_manifest_value(
                read_package.manifest,
                "Canvas 项目清单",
            )
            .map_err(|error| ProjectPackageTransferError::Message(error.to_string()))?;
        let media_payload =
            index_import_media_payload(&package.media_files, read_package.media_bytes)
                .map_err(ProjectPackageTransferError::Message)?;
        validate_canvas_package_manifest(&package, &media_payload)
            .map_err(AppError::InvalidArgument)?;
        let provider_slots = package.provider_slots.clone();
        let provider_resolutions = self.resolve_provider_slots(&provider_slots).await?;
        super::bundle_transfer::remap_canvas_provider_slots(
            &mut package,
            &provider_slots,
            &provider_resolutions,
        );
        let import_result = {
            let mut writer = VideoStudioImportMediaWriter {
                host_port,
                media_payload: &media_payload,
            };
            self.canvas
                .import_video_studio_package(
                    input.user_id,
                    package,
                    Utc::now().naive_utc(),
                    &mut writer,
                )
                .await
        };
        import_result_with_file_rollback(import_result, host_port).await
    }
}

#[async_trait::async_trait]
impl CanvasProjectTransferApplicationPort for OrchestratorContext {
    async fn export_project(
        &self,
        input: ExportCanvasProjectApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError> {
        self.export_canvas_project_validated(input, host_port).await
    }

    async fn import_project(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectSummary, ProjectPackageTransferError> {
        let user_id = input.user_id;
        let project_id = self
            .import_canvas_project_validated(input, host_port)
            .await?;
        self.project_summary_for_user(user_id, project_id)
            .await?
            .ok_or_else(|| {
                ProjectPackageTransferError::Application(a4n_error::AppError::NotFound(format!(
                    "导入后的项目不存在: {}",
                    project_id
                )))
            })
    }
}

struct VideoStudioImportMediaWriter<'a, Host: ?Sized> {
    host_port: &'a mut Host,
    media_payload: &'a ImportMediaPayload,
}

impl<Host> VideoStudioProjectImportMediaFileWriter for VideoStudioImportMediaWriter<'_, Host>
where
    Host: ProjectPackageTransferHostPort + ?Sized,
{
    fn write_imported_media_file(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        owner_id: i64,
        field: &str,
        folder: &str,
        stem: &str,
    ) -> Result<Option<String>, VideoStudioProjectImportMediaError> {
        let key = media_file_key(owner_kind, owner_id, field);
        let Some(entry) = self.media_payload.entries.get(&key) else {
            return Ok(None);
        };
        let bytes = self
            .media_payload
            .bytes
            .get(&entry.package_path)
            .ok_or_else(|| {
                VideoStudioProjectImportMediaError::Write(format!(
                    "视频画布项目包缺少媒体内容: owner_kind={owner_kind}, owner_id={owner_id}, field={field}"
                ))
            })?;
        let extension = required_package_extension(&entry.package_path)
            .map_err(VideoStudioProjectImportMediaError::Write)?;
        let relative_path =
            format!("projects/{project_id}/{folder}/{stem}_{owner_id}_{field}.{extension}");
        self.host_port
            .write_imported_media_file(&relative_path, bytes, "视频画布项目包媒体文件")
            .map_err(VideoStudioProjectImportMediaError::Write)?;
        Ok(Some(relative_path))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use a4n_domain::{canvas::CanvasNodeRecord, project::ProjectRecord};
    use serde_json::json;

    use crate::project::{ProjectBundleProviderSlot, VideoStudioProjectPackage};

    use super::super::bundle_transfer::remap_canvas_provider_slots;

    #[test]
    fn import_remaps_canvas_provider_config_ids() {
        let mut package = canvas_package();
        let slots = vec![image_provider_slot()];
        let resolutions = HashMap::from([("canvas_config:image:7".to_string(), Some(99))]);

        remap_canvas_provider_slots(&mut package, &slots, &resolutions);

        assert_eq!(package.canvas_nodes[0].config_json["image_config_id"], 99);
        assert!(
            package.canvas_nodes[0]
                .config_json
                .get("a4n_provider_slot_unresolved")
                .is_none()
        );
    }

    #[test]
    fn import_marks_canvas_provider_config_unresolved_when_missing() {
        let mut package = canvas_package();
        let slots = vec![image_provider_slot()];
        let resolutions = HashMap::from([("canvas_config:image:7".to_string(), None)]);

        remap_canvas_provider_slots(&mut package, &slots, &resolutions);

        assert!(
            package.canvas_nodes[0]
                .config_json
                .get("image_config_id")
                .is_none()
        );
        assert_eq!(
            package.canvas_nodes[0].config_json["a4n_provider_slot_unresolved"].as_bool(),
            Some(true)
        );
    }

    fn canvas_package() -> VideoStudioProjectPackage {
        let now = chrono::Utc::now().naive_utc();
        let project = ProjectRecord {
            id: 1,
            user_id: 2,
            title: "Canvas".to_string(),
            description: String::new(),
            project_type: "script".to_string(),
            project_kind: "canvas".to_string(),
            root_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".to_string(),
            status: "draft".to_string(),
            lifecycle_gate: "active".to_string(),
            target_chapters: 0,
            current_chapter: 0,
            total_chapters: None,
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: now,
            updated_at: now,
        };
        VideoStudioProjectPackage {
            version: "canvas_project_v1".to_string(),
            exported_at: String::new(),
            mode: "canvas_project".to_string(),
            project,
            canvas_graphs: Vec::new(),
            canvas_nodes: vec![CanvasNodeRecord {
                id: 3,
                graph_id: 4,
                node_kind: "GenerateImageNode".to_string(),
                ref_type: None,
                ref_id: None,
                config_json: json!({"image_config_id": 7, "model": "image-model"}),
                position_json: json!({"x": 0, "y": 0}),
                state: "idle".to_string(),
                created_at: now,
                updated_at: now,
            }],
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
            provider_slots: Vec::new(),
            media_files: Vec::new(),
        }
    }

    fn image_provider_slot() -> ProjectBundleProviderSlot {
        ProjectBundleProviderSlot {
            slot_key: "canvas_config:image:7".to_string(),
            source_config_id: Some(7),
            capability: "image".to_string(),
            provider: None,
            backend: Some("replicate".to_string()),
            model: Some("image-model".to_string()),
            base_url: None,
            parameters: json!({}),
            configuration_fingerprint_sha256: "0".repeat(64),
            requires_credentials: true,
        }
    }
}
