use a4n_core::{error::AppError, models::media::media_asset, models::project::novel_project};
use a4n_db::project_family_import::{
    ProjectFamilyMediaFileImporter, ProjectFamilyMediaImportError,
};

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::project::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    ExportedProjectPackage, FamilyImportMediaPayload, ImportProjectPackageApplicationInput,
    ProjectFamilyExportSelection, ProjectFamilyPackage, ProjectFamilyTransferHostPort,
    ProjectPackageTransferError, assemble_project_family_export_package,
    import_result_with_file_rollback, imported_project_family_media_path, media_asset_path,
    media_file_key, project_family_export_media_files, validate_family_media_manifest,
    validate_family_package_manifest,
};

impl OrchestratorContext {
    pub async fn export_project_family_package_validated<Host>(
        &self,
        input: ExportProjectFamilyPackageApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>
    where
        Host: ProjectFamilyTransferHostPort,
    {
        self.verify_project_permission(
            input.root_project_id,
            input.user_id,
            ProjectAccessRole::Owner,
        )
        .await?;
        let selection = self
            .project_family
            .export_selection(input.root_project_id, input.include_derived_projects)
            .await?;
        let project = selection.root.clone();
        let package = self
            .assemble_project_family_transfer_package(selection, "project_family")
            .await?;
        let package_bytes = host_port
            .render_export_package(&package)
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        Ok(ExportedProjectPackage {
            project,
            package_bytes,
        })
    }

    pub async fn export_derived_project_package_validated<Host>(
        &self,
        input: ExportDerivedProjectPackageApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>
    where
        Host: ProjectFamilyTransferHostPort,
    {
        self.verify_project_permission(
            input.derived_project_id,
            input.user_id,
            ProjectAccessRole::Owner,
        )
        .await?;
        let selection = self
            .project_family
            .derived_export_selection(input.derived_project_id)
            .await?;
        let project = selection.derived_projects.first().cloned().ok_or_else(|| {
            ProjectPackageTransferError::Message("派生项目导出选择为空".to_string())
        })?;
        let package = self
            .assemble_project_family_transfer_package(selection, "derived_project")
            .await?;
        let package_bytes = host_port
            .render_export_package(&package)
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        Ok(ExportedProjectPackage {
            project,
            package_bytes,
        })
    }

    pub async fn import_project_family_package_validated<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<Vec<novel_project::Model>, ProjectPackageTransferError>
    where
        Host: ProjectFamilyTransferHostPort,
    {
        if input.package_bytes.is_empty() {
            return Err(ProjectPackageTransferError::Message(
                "项目族导入包不能为空".to_string(),
            ));
        }
        let read_package = host_port
            .read_import_package(input.package_bytes)
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        validate_family_package_manifest(&read_package.package)
            .and_then(|_| {
                validate_family_media_manifest(&read_package.package, &read_package.media_payload)
            })
            .map_err(AppError::InvalidArgument)?;
        let import_package = read_package.package.into_import_package();
        let import_result = {
            let mut importer = ProjectFamilyImportMediaWriter {
                host_port,
                media_payload: &read_package.media_payload,
            };
            self.project_family
                .import_package_rows(input.user_id, &import_package, &mut importer)
                .await
        };
        import_result_with_file_rollback(import_result, host_port).await
    }

    async fn assemble_project_family_transfer_package(
        &self,
        selection: ProjectFamilyExportSelection,
        mode: &str,
    ) -> Result<ProjectFamilyPackage, ProjectPackageTransferError> {
        let derived_project_ids = selection
            .derived_projects
            .iter()
            .map(|project| project.id)
            .collect::<Vec<_>>();
        let derived_source_snapshot_ids = selection
            .derived_projects
            .iter()
            .filter_map(|project| project.source_snapshot_id)
            .collect::<Vec<_>>();
        let export_rows = self
            .project_family
            .export_rows(
                selection.root.id,
                &derived_project_ids,
                &derived_source_snapshot_ids,
            )
            .await?;
        let media_files = project_family_export_media_files(&export_rows)?;
        assemble_project_family_export_package(
            selection.root,
            selection.derived_projects,
            mode,
            export_rows,
            media_files,
        )
        .map_err(ProjectPackageTransferError::Message)
    }
}

struct ProjectFamilyImportMediaWriter<'a, Host> {
    host_port: &'a mut Host,
    media_payload: &'a FamilyImportMediaPayload,
}

impl<Host> ProjectFamilyMediaFileImporter for ProjectFamilyImportMediaWriter<'_, Host>
where
    Host: ProjectFamilyTransferHostPort,
{
    fn write_imported_media_field(
        &mut self,
        old_asset: &media_asset::Model,
        new_asset: &media_asset::Model,
        field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        let old_value =
            media_asset_path(old_asset, field).map_err(ProjectFamilyMediaImportError::Write)?;
        if old_value.is_none() {
            return Ok(None);
        }
        let entry = self
            .media_payload
            .entries
            .get(&media_file_key(old_asset.id, field))
            .ok_or_else(|| {
                ProjectFamilyMediaImportError::Write(format!(
                    "媒体资产缺少文件条目: {} {field}",
                    old_asset.id
                ))
            })?;
        let bytes = self
            .media_payload
            .bytes
            .get(&entry.package_path)
            .ok_or_else(|| {
                ProjectFamilyMediaImportError::Write(format!(
                    "媒体文件字节缺失: {}",
                    entry.package_path
                ))
            })?;
        let relative_path = imported_project_family_media_path(
            new_asset.project_id,
            &new_asset.media_kind,
            new_asset.id,
            field,
            &entry.package_path,
        )
        .map_err(ProjectFamilyMediaImportError::Write)?;
        self.host_port
            .write_imported_media_file(&relative_path, bytes, "项目族包媒体文件")
            .map_err(ProjectFamilyMediaImportError::Write)?;
        Ok(Some(relative_path))
    }
}
