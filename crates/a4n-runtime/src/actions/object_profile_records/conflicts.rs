use crate::{
    AppState,
    actions::{
        dependency_conflict::{DownstreamConflictStrategy, resolve_downstream_id_conflict},
        profile_materials,
    },
};
use a4n_orchestrator::profile_material::PROFILE_KIND_OBJECT;

pub(super) async fn object_profile_project_id(
    state: &AppState,
    object_profile_id: i64,
) -> Result<i64, String> {
    let profile = state
        .orchestrator
        .object_profile_by_id(object_profile_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(profile.project_id)
}

pub(super) async fn resolve_object_profile_downstream_conflict(
    state: &AppState,
    project_id: i64,
    object_profile_id: i64,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Vec<i64>, String> {
    let image_ids = profile_materials::list_profile_material_image_ids(
        state,
        project_id,
        PROFILE_KIND_OBJECT,
        object_profile_id,
    )
    .await?;
    resolve_downstream_id_conflict(
        image_ids,
        strategy,
        &format!("物体档案 #{object_profile_id} "),
        "张素材图",
        operation,
        "源档案",
    )
}
