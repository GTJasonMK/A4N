use super::contracts::{
    ChapterGenerationChunk, ChapterGenerationProgress, ChapterGenerationTaskScope,
    GenerateChapterStreamInput,
};
use super::preparation::prepare_chapter_generation;
use super::progress::{check_cancelled, report_progress};
use crate::AppState;
use crate::actions::access::verify_project_editor_string;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_orchestrator::chapter_generation::{
    ChapterGenerateStreamInput, StartChapterGenerationWorkflowInput, chapter_stream_task_id,
};
use tokio_util::sync::CancellationToken;

mod workflow;

pub async fn generate_chapter_stream(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterStreamInput,
    mut progress: Option<ChapterGenerationProgress<'_>>,
    mut chunk: Option<ChapterGenerationChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<String, String> {
    generate_chapter_stream_task_scope(state, user_id, &input).await?;
    let task_id = chapter_stream_task_id(input.project_id, input.chapter_number, input.task_id)?;
    let progress_total = 1;

    report_progress(&mut progress, 0, progress_total, "正在读取章节大纲")?;
    check_cancelled(&cancel_token)?;
    let prepared = prepare_chapter_generation(
        state,
        input.project_id,
        input.chapter_number,
        input.writing_notes.as_deref(),
        input.min_word_count,
        input.max_word_count,
        input.client_rag_context.as_deref(),
        &mut progress,
        progress_total,
        Some(&cancel_token),
    )
    .await?;

    report_progress(&mut progress, 0, progress_total, "正在流式生成正文")?;
    check_cancelled(&cancel_token)?;
    let workflow_run_id = state
        .orchestrator
        .start_chapter_generation_workflow(StartChapterGenerationWorkflowInput {
            project_id: input.project_id,
            chapter_number: input.chapter_number,
            task_id: &task_id,
            chapter_title: &prepared.chapter_title,
            outline_text: &prepared.outline_text,
            context_before: prepared.context_before.as_deref(),
            word_count_range: prepared.word_count_range,
            use_rag: prepared.use_rag,
            variables: &prepared.variables,
        })
        .await?;
    let chunk_cancel_token = cancel_token.clone();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let result = state
        .orchestrator
        .generate_chapter_stream(ChapterGenerateStreamInput {
            project_id: input.project_id,
            chapter_number: input.chapter_number,
            chapter_outline: &prepared.outline_text,
            context_before: prepared.context_before.as_deref(),
            prompt_renderer: &prompt_renderer,
            variables: &prepared.variables,
            workflow_run_id,
            cancel_token: Some(cancel_token.clone()),
            on_chunk: move |delta, index| {
                if delta.is_empty() {
                    return;
                }
                if let Some(chunk) = chunk.as_deref_mut()
                    && chunk(delta, index).is_err()
                {
                    chunk_cancel_token.cancel();
                    return;
                }
            },
        })
        .await;

    workflow::finish_chapter_generation_workflow(
        state,
        &mut progress,
        progress_total,
        task_id,
        workflow_run_id,
        result,
    )
    .await
}

pub async fn generate_chapter_stream_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterStreamInput,
) -> Result<ChapterGenerationTaskScope, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .orchestrator
        .ensure_text_source_project(input.project_id, "生成章节正文")
        .await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    Ok(ChapterGenerationTaskScope {
        project_id: input.project_id,
        total: 1,
    })
}
