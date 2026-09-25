use a4n_api_contract::project_transfer::{
    ProjectPackageExportInput, ProjectPackageImportPayload, ProjectPackageTransferHostPort,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::{AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project};
use a4n_storage::write_new_data_file;

use super::package_files::write_project_package_zip;

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
    async fn rollback_imported_project(&self, user_id: i64, project_id: i64) -> Result<(), String> {
        crate::actions::project_records::delete_project(self.state, user_id, project_id).await
    }

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
        mut input: ProjectPackageExportInput,
    ) -> Result<Vec<u8>, String> {
        let entries = super::package_files::project_package_media_zip_entries(
            self.state.data_dir(),
            input
                .media_files
                .iter()
                .map(|media| (media.original_path.as_str(), media.package_path.as_str())),
            &input.package_label,
        )?;
        let mut media_sha256 = serde_json::Map::new();
        for entry in &entries {
            let digest = Sha256::digest(&entry.bytes);
            media_sha256.insert(
                entry.path.clone(),
                serde_json::Value::String(
                    digest.iter().map(|byte| format!("{byte:02x}")).collect(),
                ),
            );
        }
        if !media_sha256.is_empty()
            && let Some(manifest) = input.manifest.as_object_mut()
        {
            manifest.insert(
                "media_sha256".to_string(),
                serde_json::Value::Object(media_sha256),
            );
        }
        // Project packages may contain large videos. Keep raw media out of the
        // sidecar JSON transport; the Host owns this archive boundary.
        let manifest = serde_json::to_vec_pretty(&input.manifest)
            .map_err(|error| format!("{}序列化失败：{error}", input.manifest_label))?;
        write_project_package_zip(
            input.manifest_path,
            manifest,
            input.manifest_label,
            entries,
            input.package_label,
        )
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
