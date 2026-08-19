use super::{
    CreateSceneProfileInput, DeleteSceneProfileInput, SceneProfileDeleteResult,
    SceneProfileMutationResult, UpdateSceneProfileInput, conflicts, draft, projections,
};
use crate::{
    AppState,
    actions::{
        access,
        dependency_conflict::{
            delete_downstream_images_if_requested, downstream_cleanup_warnings,
        },
    },
};

pub async fn create_scene_profile(
    state: &AppState,
    user_id: i64,
    input: CreateSceneProfileInput,
) -> Result<SceneProfileMutationResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = state
        .orchestrator()
        .create_scene_profile_validated(input.project_id, draft::into_draft(input.profile))
        .await
        .map_err(|error| error.to_string())?;
    Ok(SceneProfileMutationResult {
        profile: projections::to_summary(state.orchestrator(), &profile).await?,
        warnings: scene_rag_sync_warnings(state, input.project_id).await,
    })
}

pub async fn update_scene_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateSceneProfileInput,
) -> Result<SceneProfileMutationResult, String> {
    let scene_profile = state.orchestrator();
    let project_id =
        conflicts::scene_profile_project_id(&scene_profile, input.scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_scene_profile_downstream_conflict(
        state.orchestrator(),
        project_id,
        input.scene_profile_id,
        input.downstream_strategy,
        "修改",
    )
    .await?;
    let profile = scene_profile
        .update_scene_profile_validated(input.scene_profile_id, draft::into_draft(input.profile))
        .await
        .map_err(|error| error.to_string())?;
    let mut warnings = downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, downstream).await,
    );
    warnings.extend(scene_rag_sync_warnings(state, project_id).await);
    Ok(SceneProfileMutationResult {
        profile: projections::to_summary(scene_profile, &profile).await?,
        warnings,
    })
}

pub async fn delete_scene_profile(
    state: &AppState,
    user_id: i64,
    input: DeleteSceneProfileInput,
) -> Result<SceneProfileDeleteResult, String> {
    let scene_profile = state.orchestrator();
    let project_id =
        conflicts::scene_profile_project_id(&scene_profile, input.scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_scene_profile_downstream_conflict(
        state.orchestrator(),
        project_id,
        input.scene_profile_id,
        input.downstream_strategy,
        "删除",
    )
    .await?;
    scene_profile
        .delete_scene_profile_records_validated(input.scene_profile_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut warnings = downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, downstream).await,
    );
    warnings.extend(scene_rag_sync_warnings(state, project_id).await);
    Ok(SceneProfileDeleteResult { warnings })
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
