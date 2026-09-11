use super::super::{ProfileMaterialGroup, UploadProfileMaterialImageInput};

use crate::{AppState, actions::image_generation::RuntimeImageGenerationHostPort};

pub async fn upload_profile_material_image(
    state: &AppState,
    user_id: i64,
    input: UploadProfileMaterialImageInput,
) -> Result<ProfileMaterialGroup, String> {
    let host = RuntimeImageGenerationHostPort::new(state);
    state
        .profile_material_upload_application()
        .upload(user_id, &host, input)
        .await
        .map_err(|error| error.to_string())
}
