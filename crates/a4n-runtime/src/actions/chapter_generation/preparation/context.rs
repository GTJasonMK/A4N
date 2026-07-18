use super::super::contracts::ChapterGenerationProgress;
use super::super::generation_context::{GenerationRagContext, build_generation_rag_context};
use super::super::progress::{check_optional_cancelled, report_progress};
use crate::AppState;
use a4n_core::models::chapter::chapter_outline;
use tokio_util::sync::CancellationToken;

pub(super) struct ChapterGenerationContextInput<'a, 'progress> {
    pub(super) state: &'a AppState,
    pub(super) project_id: i64,
    pub(super) chapter_number: i32,
    pub(super) outline: &'a chapter_outline::Model,
    pub(super) rag_query_outline_text: &'a str,
    pub(super) writing_instruction: Option<&'a str>,
    pub(super) client_rag_context: Option<&'a str>,
    pub(super) progress: &'a mut Option<ChapterGenerationProgress<'progress>>,
    pub(super) progress_total: u32,
    pub(super) cancel_token: Option<&'a CancellationToken>,
}

pub(super) struct PreparedChapterGenerationContext {
    pub(super) reference_context: Option<String>,
    pub(super) rag_context: Option<String>,
    pub(super) context_before: Option<String>,
}

struct GenerationRagInput<'a> {
    state: &'a AppState,
    project_id: i64,
    chapter_number: i32,
    outline: &'a chapter_outline::Model,
    rag_query_outline_text: &'a str,
    writing_instruction: Option<&'a str>,
    client_rag_context: Option<&'a str>,
}

impl PreparedChapterGenerationContext {
    pub(super) fn append_to_outline(&self, outline_text: &mut String) {
        for context in [
            self.reference_context.as_deref(),
            self.rag_context.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            outline_text.push_str("\n\n");
            outline_text.push_str(context);
        }
    }
}

pub(super) async fn prepare_chapter_generation_context(
    input: ChapterGenerationContextInput<'_, '_>,
) -> Result<PreparedChapterGenerationContext, String> {
    report_progress(
        input.progress,
        0,
        input.progress_total,
        "正在准备章节参考资料",
    )?;
    check_optional_cancelled(input.cancel_token)?;
    let reference_context = input
        .state
        .orchestrator
        .resolve_generation_reference_context(
            input.project_id,
            input.chapter_number,
            Some(input.outline),
        )
        .await?;

    report_progress(
        input.progress,
        0,
        input.progress_total,
        "正在检索相关 RAG 上下文",
    )?;
    check_optional_cancelled(input.cancel_token)?;
    let rag_context = resolve_rag_context(GenerationRagInput {
        state: input.state,
        project_id: input.project_id,
        chapter_number: input.chapter_number,
        outline: input.outline,
        rag_query_outline_text: input.rag_query_outline_text,
        writing_instruction: input.writing_instruction,
        client_rag_context: input.client_rag_context,
    })
    .await?;

    report_progress(
        input.progress,
        0,
        input.progress_total,
        "正在收集前文上下文",
    )?;
    check_optional_cancelled(input.cancel_token)?;
    let context_before = input
        .state
        .orchestrator
        .resolve_chapter_context_before(input.project_id, input.chapter_number, None)
        .await
        .map_err(|error| format!("构建章节前文上下文失败: {error}"))?;

    Ok(PreparedChapterGenerationContext {
        reference_context,
        rag_context,
        context_before,
    })
}

async fn resolve_rag_context(input: GenerationRagInput<'_>) -> Result<Option<String>, String> {
    if input.chapter_number <= 1 {
        return Ok(None);
    }
    if let Some(context) = input
        .client_rag_context
        .map(str::trim)
        .filter(|context| !context.is_empty())
    {
        return Ok(Some(context.to_string()));
    }

    build_generation_rag_context(
        input.state,
        input.project_id,
        input.chapter_number,
        Some(input.outline),
        input.rag_query_outline_text,
        input.writing_instruction,
    )
    .await
    .map(|context: GenerationRagContext| context.text)
    .map_err(|error| format!("构建章节生成检索参考失败: {error}"))
}
