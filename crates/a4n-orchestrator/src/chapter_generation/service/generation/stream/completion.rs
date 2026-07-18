use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_llm::provider::ChatRequest;

use crate::chapter_generation::service::ChapterGenerationService;
use crate::chapter_generation::service::generation_lifecycle::{
    ChapterGenerationSession, ChapterGenerationTrace,
};

pub(super) struct StreamCompletionInput<'a> {
    pub(super) service: &'a ChapterGenerationService,
    pub(super) session: &'a ChapterGenerationSession,
    pub(super) content: &'a str,
    pub(super) chapter_outline: &'a str,
    pub(super) request_trace: ChatRequest,
    pub(super) finish_reason: Option<String>,
    pub(super) model_name: String,
}

pub(super) async fn finish_stream_chapter(
    input: StreamCompletionInput<'_>,
) -> Result<chapter::Model, AppError> {
    let StreamCompletionInput {
        service,
        session,
        content,
        chapter_outline,
        request_trace,
        finish_reason,
        model_name,
    } = input;
    service
        .finish_generated_chapter(
            session,
            content,
            chapter_outline,
            "流式生成",
            Some(ChapterGenerationTrace {
                request: request_trace,
                finish_reason,
                model_name,
            }),
        )
        .await
}
