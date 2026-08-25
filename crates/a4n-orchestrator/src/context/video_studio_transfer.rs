use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::canvas::{VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter};
use chrono::Utc;

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::project::{
    ExportVideoStudioProjectApplicationInput, ExportedProjectPackage, ImportMediaPayload,
    ImportProjectPackageApplicationInput, ProjectPackageTransferError,
    VideoStudioProjectTransferHostPort, assemble_video_studio_export_package,
    import_result_with_file_rollback, media_file_key, required_package_extension,
    validate_package_manifest, video_studio_export_media_files,
};

impl OrchestratorContext {
    pub async fn export_video_studio_project_validated<Host>(
        &self,
        input: ExportVideoStudioProjectApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>
    where
        Host: VideoStudioProjectTransferHostPort,
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
        let package = assemble_video_studio_export_package(project.clone(), rows, media_files)?;
        let package_bytes = host_port
            .render_export_package(&package)
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        Ok(ExportedProjectPackage {
            project,
            package_bytes,
        })
    }

    pub async fn import_video_studio_project_validated<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<novel_project::Model, ProjectPackageTransferError>
    where
        Host: VideoStudioProjectTransferHostPort,
    {
        if input.package_bytes.is_empty() {
            return Err(ProjectPackageTransferError::Message(
                "视频画布项目包不能为空".to_string(),
            ));
        }
        let read_package = host_port
            .read_import_package(input.package_bytes)
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        validate_package_manifest(&read_package.package, &read_package.media_payload)
            .map_err(AppError::InvalidArgument)?;
        let import_result = {
            let mut writer = VideoStudioImportMediaWriter {
                host_port,
                media_payload: &read_package.media_payload,
            };
            self.canvas
                .import_video_studio_package(
                    input.user_id,
                    read_package.package,
                    Utc::now().naive_utc(),
                    &mut writer,
                )
                .await
        };
        import_result_with_file_rollback(import_result, host_port).await
    }
}

struct VideoStudioImportMediaWriter<'a, Host> {
    host_port: &'a mut Host,
    media_payload: &'a ImportMediaPayload,
}

impl<Host> VideoStudioProjectImportMediaFileWriter for VideoStudioImportMediaWriter<'_, Host>
where
    Host: VideoStudioProjectTransferHostPort,
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
