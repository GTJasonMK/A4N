use super::{ProfileMaterialGroup, SetProfilePrimaryMaterialInput};
use crate::AppState;

pub async fn set_profile_primary_material(
    state: &AppState,
    user_id: i64,
    input: SetProfilePrimaryMaterialInput,
) -> Result<ProfileMaterialGroup, String> {
    state
        .orchestrator()
        .set_profile_primary_material_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
