use a4n_error::AppError;

use crate::{AppState, actions::access};

use super::{BlueprintMutationResult, SaveBlueprintInput, VersionedBlueprintMutationResult};

pub async fn save_blueprint(
    state: &AppState,
    user_id: i64,
    input: SaveBlueprintInput,
) -> Result<VersionedBlueprintMutationResult, AppError> {
    let project_id = input.project_id;
    let mut result = state
        .blueprint_mutation_application()
        .save(user_id, input)
        .await?;
    result
        .warnings
        .extend(blueprint_material_sync_warnings(state, project_id).await);
    Ok(result)
}

pub async fn sync_blueprint_materials(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<BlueprintMutationResult, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_material_sync_warnings(state, project_id).await,
    })
}

pub(crate) async fn blueprint_material_sync_warnings(
    state: &AppState,
    project_id: i64,
) -> Vec<String> {
    crate::rag_ingest::sync_blueprint_material_rag_records(state, project_id)
        .await
        .err()
        .map(|error| {
            format!("资料检索索引未完全更新：{error}。正式资料未受影响，可稍后在项目操作中重新同步")
        })
        .into_iter()
        .collect()
}
