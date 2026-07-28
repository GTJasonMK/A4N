use std::collections::BTreeMap;

use crate::state::ProfileMaterialRuntimeFacade;

pub(crate) async fn count_profile_material_images(
    profile_material: ProfileMaterialRuntimeFacade<'_>,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<i64, String> {
    profile_material
        .count_profile_material_images(project_id, kind, profile_id)
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn count_profile_material_images_by_profile(
    profile_material: ProfileMaterialRuntimeFacade<'_>,
    project_id: i64,
    kind: &str,
    profile_ids: &[i64],
) -> Result<BTreeMap<i64, i64>, String> {
    profile_material
        .count_profile_material_images_by_profile(project_id, kind, profile_ids)
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn list_profile_material_image_ids(
    profile_material: ProfileMaterialRuntimeFacade<'_>,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<Vec<i64>, String> {
    profile_material
        .profile_material_image_ids(project_id, kind, profile_id)
        .await
        .map_err(|error| error.to_string())
}
