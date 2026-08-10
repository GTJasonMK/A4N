use a4n_orchestrator::profile_material::{ProfileMaterialGroup, UploadProfileMaterialImageInput};

use crate::AppState;

pub async fn upload_profile_material_image(
    state: &AppState,
    user_id: i64,
    input: UploadProfileMaterialImageInput,
) -> Result<ProfileMaterialGroup, String> {
    state
        .profile_material()
        .upload_profile_material_image_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
