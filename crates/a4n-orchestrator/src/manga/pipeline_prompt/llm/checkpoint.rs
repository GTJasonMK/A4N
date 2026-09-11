use a4n_error::AppError;
use serde_json::Value;

use crate::OrchestratorContext;

use super::super::contracts::MangaRestoredLlmCheckpoint;

pub(in crate::manga) async fn load_manga_restored_llm_checkpoint(
    context: &OrchestratorContext,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<MangaRestoredLlmCheckpoint>, AppError> {
    let Some(checkpoint) = context
        .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
        .await?
    else {
        return Ok(None);
    };
    let usage = checkpoint
        .usage
        .map(serde_json::from_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    Ok(Some(MangaRestoredLlmCheckpoint {
        response_text: checkpoint.response_content,
        parsed_output: checkpoint.parsed_output,
        usage,
        finish_reason: checkpoint.finish_reason,
    }))
}
