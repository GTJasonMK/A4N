use a4n_api_contract::project_transfer::{
    ProjectPackageExportInput, ProjectPackageImportPayload, ProjectPackageTransferHostPort,
};
use async_trait::async_trait;

use crate::{AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project};
use a4n_storage::write_new_data_file;

pub(super) struct RuntimeProjectPackageTransferHostPort<'a> {
    state: &'a AppState,
    written_paths: Vec<String>,
    cleanup_token: String,
}

impl<'a> RuntimeProjectPackageTransferHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self {
            state,
            written_paths: Vec::new(),
            cleanup_token: uuid::Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait]
impl ProjectPackageTransferHostPort for RuntimeProjectPackageTransferHostPort<'_> {
    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), String> {
        write_new_data_file(self.state.data_dir(), relative_path, bytes, label)?;
        self.written_paths.push(relative_path.to_string());
        Ok(())
    }

    async fn rollback_imported_media_files(&mut self) -> Result<(), String> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &self.cleanup_token,
            &self.written_paths,
        )
        .await
    }

    async fn render_export_package(
        &self,
        input: ProjectPackageExportInput,
    ) -> Result<Vec<u8>, String> {
        let manifest = crate::sidecar_clients::render_export_json_manifest_bytes(
            self.state.runtime_profile(),
            self.state.capabilities().export.clone(),
            &input.manifest,
            &input.manifest_label,
            input.project_id,
            input.root_project_id,
        )
        .await?;
        let entries = super::package_files::project_package_media_zip_entries(
            self.state.data_dir(),
            input
                .media_files
                .iter()
                .map(|media| (media.original_path.as_str(), media.package_path.as_str())),
            &input.package_label,
        )?;
        crate::sidecar_clients::render_export_zip_package(
            self.state.runtime_profile(),
            self.state.capabilities().export.clone(),
            a4n_export_contract::ExportZipPackageInput {
                manifest_path: input.manifest_path,
                manifest_bytes: manifest,
                manifest_label: input.manifest_label,
                entries,
                package_label: input.package_label,
            },
            input.project_id,
            input.root_project_id,
        )
        .await
    }

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
        manifest_path: &str,
        package_label: &str,
        manifest_label: &str,
    ) -> Result<ProjectPackageImportPayload, String> {
        let (manifest, media_bytes) = crate::sidecar_clients::read_import_media_package_zip(
            self.state.runtime_profile(),
            self.state.capabilities().import.clone(),
            package_bytes,
            manifest_path,
            package_label,
            manifest_label,
        )
        .await?;
        Ok(ProjectPackageImportPayload {
            manifest,
            media_bytes,
        })
    }
}
