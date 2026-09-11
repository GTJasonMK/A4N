use super::{
    CreateSceneProfileInput, DeleteSceneProfileInput, SceneProfileDeleteResult,
    SceneProfileMutationResult, UpdateSceneProfileInput,
};
use crate::{
    AppState,
    actions::dependency_conflict::{
        delete_downstream_images_if_requested, downstream_cleanup_warnings,
    },
};

pub async fn create_scene_profile(
    state: &AppState,
    user_id: i64,
    input: CreateSceneProfileInput,
) -> Result<SceneProfileMutationResult, String> {
    let project_id = input.project_id;
    let outcome = state
        .scene_profile_mutation_application()
        .create(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut result = outcome.result;
    result.warnings.extend(downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, outcome.downstream_image_ids).await,
    ));
    result
        .warnings
        .extend(scene_rag_sync_warnings(state, project_id).await);
    Ok(result)
}

pub async fn update_scene_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateSceneProfileInput,
) -> Result<SceneProfileMutationResult, String> {
    let outcome = state
        .scene_profile_mutation_application()
        .update(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut result = outcome.result;
    result.warnings.extend(downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, outcome.downstream_image_ids).await,
    ));
    result
        .warnings
        .extend(scene_rag_sync_warnings(state, result.profile.project_id).await);
    Ok(result)
}

pub async fn delete_scene_profile(
    state: &AppState,
    user_id: i64,
    input: DeleteSceneProfileInput,
) -> Result<SceneProfileDeleteResult, String> {
    let outcome = state
        .scene_profile_mutation_application()
        .delete(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    let project_id = outcome.project_id;
    let mut result = outcome.result;
    result.warnings.extend(downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, outcome.downstream_image_ids).await,
    ));
    result
        .warnings
        .extend(scene_rag_sync_warnings(state, project_id).await);
    Ok(result)
}

async fn scene_rag_sync_warnings(state: &AppState, project_id: i64) -> Vec<String> {
    crate::rag_ingest::sync_scene_rag_records(
        state,
        project_id,
        crate::rag_ingest::SceneRagSyncTraceSource::Independent,
    )
    .await
    .err()
    .map(|error| {
        format!("场景资料检索索引未完全更新：{error}。正式资料未受影响，可稍后在项目操作中重新同步")
    })
    .into_iter()
    .collect()
}
