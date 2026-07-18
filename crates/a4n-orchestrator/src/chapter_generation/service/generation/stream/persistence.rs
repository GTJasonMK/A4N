use a4n_core::error::AppError;
use serde_json::Value;

use super::super::checkpoint::{
    ChapterGenerationCheckpointSaveInput, save_completed_chapter_generation_checkpoint,
};
use crate::chapter_generation::service::generation_lifecycle::ChapterGenerationSession;
use crate::chapter_generation::service::{
    ChapterGenerationService, ensure_chapter_response_not_truncated,
};

pub(super) struct StreamCheckpointPersistenceInput<'a> {
    pub(super) service: &'a ChapterGenerationService,
    pub(super) session: &'a ChapterGenerationSession,
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) chapter_number: i32,
    pub(super) chapter_outline: &'a str,
    pub(super) input_fingerprint_json: Value,
    pub(super) response_content: &'a str,
    pub(super) finish_reason: Option<&'a str>,
    pub(super) restored_from_checkpoint: bool,
}

pub(super) async fn validate_and_persist_stream_checkpoint(
    input: StreamCheckpointPersistenceInput<'_>,
) -> Result<(), AppError> {
    let StreamCheckpointPersistenceInput {
        service,
        session,
        workflow_run_id,
        call_key,
        chapter_number,
        chapter_outline,
        input_fingerprint_json,
        response_content,
        finish_reason,
        restored_from_checkpoint,
    } = input;
    ensure_chapter_response_not_truncated(finish_reason, &format!("第 {chapter_number} 章正文"))?;
    let validated = service.validate_generated_chapter_for_checkpoint(
        session,
        response_content,
        chapter_outline,
    )?;
    if restored_from_checkpoint {
        return Ok(());
    }
    save_completed_chapter_generation_checkpoint(ChapterGenerationCheckpointSaveInput {
        service,
        workflow_run_id,
        call_key,
        chapter_number,
        input_fingerprint_json,
        response_content,
        parsed_content: &validated.normalized_content,
        finish_reason,
    })
    .await
}
