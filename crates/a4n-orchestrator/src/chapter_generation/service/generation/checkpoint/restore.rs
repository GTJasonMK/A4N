use a4n_core::error::AppError;
use a4n_db::workflow;
use serde_json::Value;

use crate::chapter_generation::service::ChapterGenerationService;
use crate::workflow::{workflow_checkpoint_contract_app_error, workflow_read_app_error};

mod contract;

use contract::parse_completed_chapter_generation_checkpoint;

pub struct ChapterGenerationCheckpoint {
    pub(in crate::chapter_generation::service) response_content: String,
    pub(in crate::chapter_generation::service) parsed_content: String,
    pub(in crate::chapter_generation::service) finish_reason: Option<String>,
}

pub async fn load_completed_chapter_generation_checkpoint(
    service: &ChapterGenerationService,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<ChapterGenerationCheckpoint>, AppError> {
    let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        service.db.as_ref(),
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
    parse_completed_chapter_generation_checkpoint(&checkpoint_json, call_key).map(Some)
}
