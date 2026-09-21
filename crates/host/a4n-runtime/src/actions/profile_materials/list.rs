use super::{ListProfileMaterialsInput, ProfileMaterialGroup};
use crate::AppState;

pub async fn list_profile_materials(
    state: &AppState,
    user_id: i64,
    input: ListProfileMaterialsInput,
) -> Result<Vec<ProfileMaterialGroup>, String> {
    state
        .profile_material_read_application()
        .list(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
