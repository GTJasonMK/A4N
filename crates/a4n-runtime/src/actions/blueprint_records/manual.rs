use a4n_core::error::AppError;

use crate::{AppState, actions::access};

use super::{BlueprintMutationResult, SaveBlueprintInput};

pub async fn save_blueprint(
    state: &AppState,
    user_id: i64,
    input: SaveBlueprintInput,
) -> Result<BlueprintMutationResult, AppError> {
    let project_id = input.project_id;
    state
        .orchestrator()
        .save_blueprint_for_user_validated(user_id, input)
        .await?;
    Ok(BlueprintMutationResult {
        warnings: blueprint_material_sync_warnings(state, project_id).await,
    })
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

pub(super) async fn blueprint_material_sync_warnings(
    state: &AppState,
    project_id: i64,
) -> Vec<String> {
    let mut errors = Vec::new();
    collect_sync_errors(
        &mut errors,
        crate::rag_ingest::ingest_blueprints_inner(state, project_id, None).await,
    );
    collect_sync_errors(
        &mut errors,
        crate::rag_ingest::ingest_blueprint_metadata_inner(state, project_id, None).await,
    );
    collect_sync_errors(
        &mut errors,
        crate::rag_ingest::ingest_world_setting_inner(state, project_id, None).await,
    );
    collect_sync_errors(
        &mut errors,
        crate::rag_ingest::ingest_character_records_inner(state, project_id, None).await,
    );
    collect_sync_errors(
        &mut errors,
        crate::rag_ingest::ingest_relationships_inner(state, project_id, None).await,
    );

    if errors.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "资料检索索引未完全更新：{}。正式资料未受影响，可稍后在项目操作中重新同步",
            errors.into_iter().take(8).collect::<Vec<_>>().join("；")
        )]
    }
}

fn collect_sync_errors(
    errors: &mut Vec<String>,
    result: Result<crate::rag_ingest::IngestStats, String>,
) {
    match result {
        Ok(stats) => errors.extend(stats.errors),
        Err(error) => errors.push(error),
    }
}
