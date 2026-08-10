use a4n_core::error::AppError;

use crate::{AppState, rag_ingest::sync_blueprint_rag_records};

use super::{BlueprintMutationResult, SaveBlueprintInput, blueprint_rag_warnings};

pub async fn save_blueprint(
    state: &AppState,
    user_id: i64,
    input: SaveBlueprintInput,
) -> Result<BlueprintMutationResult, AppError> {
    let project_id = input.project_id;
    state
        .project_blueprint()
        .save_blueprint_for_user_validated(user_id, input)
        .await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_rag_warnings(sync_blueprint_rag_records(state, project_id).await.err()),
    })
}
