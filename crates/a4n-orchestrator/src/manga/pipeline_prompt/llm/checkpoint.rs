use a4n_core::error::AppError;
use a4n_db::{DbHandle, workflow};
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::Value;

use crate::workflow::{workflow_checkpoint_contract_app_error, workflow_read_app_error};

use super::super::contracts::MangaRestoredLlmCheckpoint;

pub(in crate::manga) async fn load_manga_restored_llm_checkpoint(
    db: &DbHandle,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
    prompt_name: &str,
) -> Result<Option<MangaRestoredLlmCheckpoint>, AppError> {
    let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        db.connection(),
        workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    else {
        return Ok(None);
    };
    let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, call_key)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let response_text =
        manga_llm_checkpoint_string(&checkpoint_json, call_key, "response_content")?;
    let finish_reason = manga_llm_checkpoint_optional_string(&checkpoint_json, "finish_reason")?;
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} checkpoint 标记为截断响应，不能用于恢复"
        )));
    }
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key}.parsed_output 缺失")))?;
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    Ok(Some(MangaRestoredLlmCheckpoint {
        response_text,
        parsed_output,
        usage,
        finish_reason,
    }))
}

fn manga_llm_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key}.{field} 必须是字符串")))
}

fn manga_llm_checkpoint_optional_string(
    checkpoint_json: &Value,
    field: &str,
) -> Result<Option<String>, AppError> {
    match checkpoint_json.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是字符串或 null"))),
    }
}
