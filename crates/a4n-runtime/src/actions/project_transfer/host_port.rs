use a4n_orchestrator::project::{
    ProjectFamilyPackage, ProjectFamilyTransferHostPort, ProjectPackageFileHostPort,
    ReadProjectFamilyPackage, ReadVideoStudioProjectPackage, VideoStudioProjectPackage,
    VideoStudioProjectTransferHostPort,
};
use async_trait::async_trait;

use crate::{
    AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::write_new_data_file,
};

use super::{
    family_package::{
        package_zip_bytes as package_family_zip_bytes, read_project_family_package_bytes,
    },
    video_studio_package::{
        package_zip_bytes as package_video_studio_zip_bytes, read_video_studio_package_bytes,
    },
};

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
impl ProjectPackageFileHostPort for RuntimeProjectPackageTransferHostPort<'_> {
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
}

#[async_trait]
impl ProjectFamilyTransferHostPort for RuntimeProjectPackageTransferHostPort<'_> {
    async fn render_export_package(
        &self,
        package: &ProjectFamilyPackage,
    ) -> Result<Vec<u8>, String> {
        package_family_zip_bytes(
            self.state.runtime_profile(),
            self.state.capabilities(),
            self.state.data_dir(),
            package,
        )
        .await
    }

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
    ) -> Result<ReadProjectFamilyPackage, String> {
        let (package, media_payload) = read_project_family_package_bytes(
            self.state.runtime_profile(),
            self.state.capabilities(),
            package_bytes,
        )
        .await?;
        Ok(ReadProjectFamilyPackage {
            package,
            media_payload,
        })
    }
}

#[async_trait]
impl VideoStudioProjectTransferHostPort for RuntimeProjectPackageTransferHostPort<'_> {
    async fn render_export_package(
        &self,
        package: &VideoStudioProjectPackage,
    ) -> Result<Vec<u8>, String> {
        let root_project_id =
            crate::actions::sidecar_root_project_id(self.state, package.project.id).await?;
        package_video_studio_zip_bytes(
            self.state.runtime_profile(),
            self.state.capabilities(),
            self.state.data_dir(),
            root_project_id,
            package,
        )
        .await
    }

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
    ) -> Result<ReadVideoStudioProjectPackage, String> {
        let (package, media_payload) = read_video_studio_package_bytes(
            self.state.runtime_profile(),
            self.state.capabilities(),
            package_bytes,
        )
        .await?;
        Ok(ReadVideoStudioProjectPackage {
            package,
            media_payload,
        })
    }
}
