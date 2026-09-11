use a4n_db::workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};
use a4n_error::AppError;
use serde_json::{Value, json};

use crate::chapter_generation::service::ChapterGenerationService;
use crate::workflow::workflow_write_app_error;

pub(in crate::chapter_generation) struct ChapterGenerationCheckpointSaveInput<'a> {
    pub(in crate::chapter_generation) service: &'a ChapterGenerationService,
    pub(in crate::chapter_generation) workflow_run_id: i64,
    pub(in crate::chapter_generation) call_key: String,
    pub(in crate::chapter_generation) chapter_number: i32,
    pub(in crate::chapter_generation) input_fingerprint_json: Value,
    pub(in crate::chapter_generation) response_content: &'a str,
    pub(in crate::chapter_generation) parsed_content: &'a str,
    pub(in crate::chapter_generation) finish_reason: Option<&'a str>,
}

pub(in crate::chapter_generation) async fn save_completed_chapter_generation_checkpoint(
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
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: json!({"content": parsed_content}),
                response_content: response_content.to_string(),
                usage: None,
                finish_reason: finish_reason.map(str::to_owned),
                artifacts_json: None,
            },
        },
    )
    .await
    .map_err(workflow_write_app_error)
}
