use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::Value;

use crate::chapter_generation::service::ChapterGenerationService;
use crate::workflow::workflow_write_app_error;

mod contract;

use contract::completed_chapter_generation_checkpoint_json;

pub struct ChapterGenerationCheckpointSaveInput<'a> {
    pub(in crate::chapter_generation::service) service: &'a ChapterGenerationService,
    pub(in crate::chapter_generation::service) workflow_run_id: i64,
    pub(in crate::chapter_generation::service) call_key: String,
    pub(in crate::chapter_generation::service) chapter_number: i32,
    pub(in crate::chapter_generation::service) input_fingerprint_json: Value,
    pub(in crate::chapter_generation::service) response_content: &'a str,
    pub(in crate::chapter_generation::service) parsed_content: &'a str,
    pub(in crate::chapter_generation::service) finish_reason: Option<&'a str>,
}

pub async fn save_completed_chapter_generation_checkpoint(
    input: ChapterGenerationCheckpointSaveInput<'_>,
) -> Result<(), AppError> {
    let ChapterGenerationCheckpointSaveInput {
        service,
        workflow_run_id,
        call_key,
        chapter_number,
        input_fingerprint_json,
        response_content,
        parsed_content,
        finish_reason,
    } = input;
    workflow::save_completed_llm_call_checkpoint(
        service.db.as_ref(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: chapter_number,
            input_fingerprint_json,
            checkpoint_json: completed_chapter_generation_checkpoint_json(
                response_content,
                parsed_content,
                finish_reason,
            ),
            artifacts_json: None,
        },
    )
    .await
    .map_err(workflow_write_app_error)
}
