use a4n_api_contract::project_transfer::{
    ExportProjectBundleApplicationInput, ExportedProjectPackage,
    ImportProjectPackageApplicationInput, ProjectBundleImportReport,
    ProjectBundleTransferApplicationPort, ProjectPackageTransferError,
    ProjectPackageTransferHostPort,
};

use super::super::OrchestratorContext;

#[async_trait::async_trait]
impl ProjectBundleTransferApplicationPort for OrchestratorContext {
    async fn export_bundle(
        &self,
        input: ExportProjectBundleApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError> {
        self.export_project_bundle_validated(input, host_port).await
    }

    async fn inspect_bundle(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectBundleImportReport, ProjectPackageTransferError> {
        self.inspect_project_bundle_validated(input, host_port)
            .await
    }

    async fn import_bundle(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectBundleImportReport, ProjectPackageTransferError> {
        match self.import_project_bundle_validated(input, host_port).await {
            Ok(report) => Ok(report),
            Err(error) => match host_port.rollback_imported_media_files().await {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(ProjectPackageTransferError::Message(format!(
                    "{error}；{cleanup_error}"
                ))),
            },
        }
    }
}
