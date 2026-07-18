use crate::{AppState, actions::prompt_records::prompt_update_many_runtime};

pub(super) async fn rollback_prompt_import_after_db_error(
    state: &AppState,
    snapshots: Vec<(String, String)>,
    db_error: String,
) -> String {
    if snapshots.is_empty() {
        return db_error;
    }
    match rollback_prompt_import(state, snapshots).await {
        Ok(()) => format!("{db_error}；已回滚本次提示词导入"),
        Err(rollback_error) => {
            format!("{db_error}；提示词已更新但回滚失败：{rollback_error}")
        }
    }
}

async fn rollback_prompt_import(
    state: &AppState,
    snapshots: Vec<(String, String)>,
) -> Result<(), String> {
    if snapshots.is_empty() {
        return Ok(());
    }
    prompt_update_many_runtime(state, snapshots, "settings_import_rollback")
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[cfg(test)]
pub(super) async fn rollback_prompt_import_after_db_error_for_engine(
    engine: &dyn a4n_prompt::engine::PromptEngine,
    snapshots: Vec<(String, String)>,
    db_error: String,
) -> String {
    if snapshots.is_empty() {
        return db_error;
    }
    match rollback_prompt_import_for_engine(engine, snapshots).await {
        Ok(()) => format!("{db_error}；已回滚本次提示词导入"),
        Err(rollback_error) => {
            format!("{db_error}；提示词已更新但回滚失败：{rollback_error}")
        }
    }
}

#[cfg(test)]
async fn rollback_prompt_import_for_engine(
    engine: &dyn a4n_prompt::engine::PromptEngine,
    snapshots: Vec<(String, String)>,
) -> Result<(), String> {
    if snapshots.is_empty() {
        return Ok(());
    }
    engine
        .update_many(snapshots, "settings_import_rollback")
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}
