use a4n_db::project_family_import::{
    ProjectFamilyMediaFileImporter, ProjectFamilyMediaImportError,
};
use a4n_db_models::models::project::novel_project;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::project::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    ExportedProjectPackage, ImportMediaPayload, ImportProjectPackageApplicationInput,
    ProjectFamilyExportSelection, ProjectFamilyPackage, ProjectFamilyTransferApplicationPort,
    ProjectPackageTransferError, ProjectPackageTransferHostPort,
    assemble_project_family_export_package, import_result_with_file_rollback,
    imported_project_family_media_path, index_import_media_payload, media_file_key,
    project_family_export_media_files, project_package_export_input,
    validate_family_media_manifest, validate_family_package_manifest,
};

impl OrchestratorContext {
    pub(crate) async fn export_project_family_package_validated<Host>(
        &self,
        input: ExportProjectFamilyPackageApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
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
        let host_input = project_package_export_input(
            &package,
            &package.media_files,
            crate::project::FAMILY_PACKAGE_MANIFEST_PATH,
            "项目族清单",
            "项目族导出包",
            Some(package.root_project_id),
            Some(package.root_project_id),
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

    pub(crate) async fn export_derived_project_package_validated<Host>(
        &self,
        input: ExportDerivedProjectPackageApplicationInput,
        host_port: &Host,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
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
        let host_input = project_package_export_input(
            &package,
            &package.media_files,
            crate::project::FAMILY_PACKAGE_MANIFEST_PATH,
            "项目族清单",
            "项目族导出包",
            Some(package.root_project_id),
            Some(package.root_project_id),
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

    pub(crate) async fn import_project_family_package_validated<Host>(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut Host,
    ) -> Result<Vec<novel_project::Model>, ProjectPackageTransferError>
    where
        Host: ProjectPackageTransferHostPort + ?Sized,
    {
        if input.package_bytes.is_empty() {
            return Err(ProjectPackageTransferError::Message(
                "项目族导入包不能为空".to_string(),
            ));
        }
        let read_package = host_port
            .read_import_package(
                input.package_bytes,
                crate::project::FAMILY_PACKAGE_MANIFEST_PATH,
                "项目族导入包",
                "项目族清单",
            )
            .await
            .map_err(ProjectPackageTransferError::Message)?;
        let package: ProjectFamilyPackage =
            a4n_import_core::parse_json_manifest_value(read_package.manifest, "项目族清单")
                .map_err(|error| ProjectPackageTransferError::Message(error.to_string()))?;
        let media_payload =
            index_import_media_payload(&package.media_files, read_package.media_bytes)
                .map_err(ProjectPackageTransferError::Message)?;
        validate_family_package_manifest(&package)
            .and_then(|_| validate_family_media_manifest(&package, &media_payload))
            .map_err(AppError::InvalidArgument)?;
        let import_package = package.into_import_package();
        let import_result = {
            let mut importer = ProjectFamilyImportMediaWriter {
                host_port,
                media_payload: &media_payload,
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

#[async_trait::async_trait]
impl ProjectFamilyTransferApplicationPort for OrchestratorContext {
    async fn export_project_family(
        &self,
        input: ExportProjectFamilyPackageApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError> {
        self.export_project_family_package_validated(input, host_port)
            .await
    }

    async fn export_derived_project(
        &self,
        input: ExportDerivedProjectPackageApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError> {
        self.export_derived_project_package_validated(input, host_port)
            .await
    }

    async fn import_project_family(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<Vec<a4n_api_contract::project::ProjectSummary>, ProjectPackageTransferError> {
        let user_id = input.user_id;
        let projects = self
            .import_project_family_package_validated(input, host_port)
            .await?;
        let mut summaries = Vec::with_capacity(projects.len());
        for project in projects {
            summaries.push(
                self.project_summary_for_user(user_id, project.id)
                    .await?
                    .ok_or_else(|| {
                        ProjectPackageTransferError::Application(a4n_error::AppError::NotFound(
                            format!("导入后的项目不存在: {}", project.id),
                        ))
                    })?,
            );
        }
        Ok(summaries)
    }
}

struct ProjectFamilyImportMediaWriter<'a, Host: ?Sized> {
    host_port: &'a mut Host,
    media_payload: &'a ImportMediaPayload,
}

impl<Host> ProjectFamilyMediaFileImporter for ProjectFamilyImportMediaWriter<'_, Host>
where
    Host: ProjectPackageTransferHostPort + ?Sized,
{
    fn write_imported_media_field(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        old_owner_id: i64,
        new_owner_id: i64,
        field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        let Some(entry) =
            self.media_payload
                .entries
                .get(&media_file_key(owner_kind, old_owner_id, field))
        else {
            return Ok(None);
        };
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
            project_id,
            owner_kind,
            new_owner_id,
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
