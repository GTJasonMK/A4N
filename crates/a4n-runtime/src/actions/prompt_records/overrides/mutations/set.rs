use super::{
    super::super::{
        SetOverrideInput,
        client::{prompt_client, sidecar_get_prompt, sidecar_set_project_override},
    },
    rollback::{
        rollback_project_override_runtime_after_persist_error,
        rollback_project_override_sidecar_after_persist_error,
    },
};
use crate::{
    AppState,
    actions::{access, app_error_message},
};
use a4n_orchestrator::prompt_override::{
    normalize_project_override_identity, validate_project_override_content,
};

pub async fn prompt_set_project_override(
    state: &AppState,
    user_id: i64,
    input: SetOverrideInput,
) -> Result<(), String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let identity = normalize_project_override_identity(input.project_id, &input.name)
        .map_err(|error| error.to_string())?;
    validate_project_override_content(&input.content).map_err(|error| error.to_string())?;
    let uses_sidecar = prompt_client(state)?.is_some();
    if uses_sidecar {
        sidecar_get_prompt(state, &identity.name).await?;
    } else {
        state
            .engine
            .get(&identity.name)
            .map_err(|error| error.to_string())?;
    }
    let content = input.content;
    let previous_content = state
        .orchestrator
        .load_project_override_persisted_content(&identity)
        .await
        .map_err(app_error_message)?;
    if uses_sidecar {
        sidecar_set_project_override(state, identity.project_id, &identity.name, content.clone())
            .await?;
    } else {
        state
            .engine
            .set_project_override(identity.project_id, &identity.name, content.clone())
            .await
            .map_err(|error| error.to_string())?;
    }

    if let Err(error) = state
        .orchestrator
        .save_project_override(identity.project_id, &identity.name, &content)
        .await
    {
        let operation_error = format!("提示词覆盖持久化失败: {}", app_error_message(error));
        return Err(if uses_sidecar {
            rollback_project_override_sidecar_after_persist_error(
                state,
                &identity,
                previous_content,
                operation_error,
                "已回滚 sidecar 运行时覆盖",
                "sidecar 运行时覆盖已更新但回滚失败：",
            )
            .await
        } else {
            rollback_project_override_runtime_after_persist_error(
                state.engine.as_ref(),
                &identity,
                previous_content,
                operation_error,
                "已回滚运行时覆盖",
                "运行时覆盖已更新但回滚失败：",
            )
            .await
        });
    }
    Ok(())
}
