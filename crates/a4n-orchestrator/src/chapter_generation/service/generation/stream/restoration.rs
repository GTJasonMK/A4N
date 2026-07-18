use a4n_core::error::AppError;
use serde_json::Value;

use super::super::checkpoint::load_completed_chapter_generation_checkpoint;
use crate::chapter_generation::service::generation_lifecycle::ChapterGenerationSession;
use crate::chapter_generation::service::{
    ChapterGenerationService, ensure_chapter_response_not_truncated,
};

pub(super) async fn restore_completed_stream_checkpoint<F>(
    service: &ChapterGenerationService,
    session: ChapterGenerationSession,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
    chapter_number: i32,
    chapter_outline: &str,
    on_chunk: &mut F,
) -> Result<(ChapterGenerationSession, Option<(String, Option<String>)>), AppError>
where
    F: FnMut(&str, u32),
{
    let Some(checkpoint) = load_completed_chapter_generation_checkpoint(
        service,
        workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await?
    else {
        return Ok((session, None));
    };

    ensure_chapter_response_not_truncated(
        checkpoint.finish_reason.as_deref(),
        &format!("第 {chapter_number} 章正文"),
    )?;
    let validated = service.validate_generated_chapter_for_checkpoint(
        &session,
        &checkpoint.response_content,
        chapter_outline,
    )?;
    if checkpoint.parsed_content != validated.normalized_content {
        service.restore_generation_status(session).await;
        return Err(AppError::InvalidArgument(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        )));
    }
    if !checkpoint.response_content.trim().is_empty() {
        on_chunk(&checkpoint.response_content, 0);
    }
    Ok((
        session,
        Some((checkpoint.response_content, checkpoint.finish_reason)),
    ))
}
