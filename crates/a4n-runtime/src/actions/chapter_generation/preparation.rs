mod context;

use super::contracts::{ChapterGenerationProgress, PreparedChapterGeneration};
use crate::AppState;
use a4n_orchestrator::chapter_generation::{
    merge_generation_outline_text, non_empty_trimmed, required_generation_chapter_title,
    resolve_generation_word_count_range, upsert_chapter_generation_variables,
};
use context::{ChapterGenerationContextInput, prepare_chapter_generation_context};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

pub(super) async fn prepare_chapter_generation(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    writing_notes: Option<&str>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
    client_rag_context: Option<&str>,
    progress: &mut Option<ChapterGenerationProgress<'_>>,
    progress_total: u32,
    cancel_token: Option<&CancellationToken>,
) -> Result<PreparedChapterGeneration, String> {
    let outline = state
        .orchestrator
        .required_chapter_generation_outline(project_id, chapter_number)
        .await?;
    let writing_instruction = writing_notes.and_then(non_empty_trimmed);
    let word_count_range =
        resolve_generation_word_count_range(Some(&outline), min_word_count, max_word_count)?;
    let chapter_title = required_generation_chapter_title(chapter_number, Some(&outline), None)?;
    let rag_query_outline_text = merge_generation_outline_text(
        chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        None,
    )?;
    let mut outline_text = merge_generation_outline_text(
        chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        Some(word_count_range),
    )?;
    let use_rag = chapter_number > 1;

    let prepared_context = prepare_chapter_generation_context(ChapterGenerationContextInput {
        state,
        project_id,
        chapter_number,
        outline: &outline,
        rag_query_outline_text: &rag_query_outline_text,
        writing_instruction,
        client_rag_context,
        progress,
        progress_total,
        cancel_token,
    })
    .await?;
    prepared_context.append_to_outline(&mut outline_text);

    let mut variables = HashMap::new();
    upsert_chapter_generation_variables(
        &mut variables,
        chapter_number,
        &chapter_title,
        &outline_text,
        writing_instruction,
        prepared_context.reference_context.as_deref(),
        prepared_context.rag_context.as_deref(),
        word_count_range,
    );

    Ok(PreparedChapterGeneration {
        chapter_title,
        outline_text,
        context_before: prepared_context.context_before,
        word_count_range,
        use_rag,
        variables,
    })
}
