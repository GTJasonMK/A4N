use super::super::OrchestratorContext;
use a4n_error::AppError;

use crate::project::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
    ExportedProjectPackage, ImportProjectPackageApplicationInput,
    ProjectFamilyTransferApplicationPort, ProjectPackageTransferError,
    ProjectPackageTransferHostPort,
};

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
                        ProjectPackageTransferError::Application(AppError::NotFound(format!(
                            "导入后的项目不存在: {}",
                            project.id
                        )))
                    })?,
            );
        }
        Ok(summaries)
    }
}
