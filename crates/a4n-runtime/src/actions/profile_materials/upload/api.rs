use a4n_core::error::AppError;
use a4n_orchestrator::profile_material::{
    PersistProfileMaterialImageInput, PersistedProfileMaterialImage, ProfileMaterialGroup,
    ProfileMaterialUploadHostPort, UploadProfileMaterialImageInput,
};
use async_trait::async_trait;

use super::file::persist_uploaded_profile_material_image;
use crate::{AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project};

struct RuntimeProfileMaterialUploadHostPort<'a> {
    state: &'a AppState,
}

#[async_trait]
impl ProfileMaterialUploadHostPort for RuntimeProfileMaterialUploadHostPort<'_> {
    fn persist_profile_material_image(
        &self,
        input: PersistProfileMaterialImageInput<'_>,
    ) -> Result<PersistedProfileMaterialImage, AppError> {
        let (_, relative_path, file_size) = persist_uploaded_profile_material_image(
            self.state.data_dir(),
            input.project_id,
            input.profile_kind,
            input.profile_id,
            input.extension,
            input.bytes,
        )
        .map_err(profile_material_host_error)?;
        Ok(PersistedProfileMaterialImage {
            relative_path,
            file_size,
        })
    }

    async fn remove_uncommitted_profile_material_image(
        &self,
        relative_path: &str,
    ) -> Result<(), AppError> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!("profile-material-upload:{relative_path}"),
            &[relative_path.to_string()],
        )
        .await
        .map_err(profile_material_host_error)
    }
}

pub async fn upload_profile_material_image(
    state: &AppState,
    user_id: i64,
    input: UploadProfileMaterialImageInput,
) -> Result<ProfileMaterialGroup, String> {
    state
        .profile_material()
        .upload_profile_material_image_validated(
            &RuntimeProfileMaterialUploadHostPort { state },
            user_id,
            input,
        )
        .await
        .map_err(|error| error.to_string())
}

fn profile_material_host_error(operation: String) -> AppError {
    AppError::InvalidState {
        current: "profile_material_upload_host".to_string(),
        operation,
    }
}
