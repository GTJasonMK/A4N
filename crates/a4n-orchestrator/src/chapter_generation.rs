use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_prompt::rendering::PromptRenderer;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

mod context_before;
mod lifecycle;
mod preparation;
mod rag_context;

#[path = "chapter_generation/service/mod.rs"]
mod service;

pub use lifecycle::{StartChapterGenerationWorkflowInput, chapter_stream_task_id};
pub use preparation::{
    ChapterGenerationWordCountRange, compact_json_text, is_meaningful_json_value,
    merge_generation_outline_text, non_empty_trimmed, outline_character_names,
    required_generation_chapter_title, resolve_generation_word_count_range, truncate_text_chars,
    upsert_chapter_generation_variables,
};
pub use rag_context::{
    GenerationRagContext, GenerationRagQueries, GenerationRagStatistics,
    PreparedGenerationRagSearch, WeightedGenerationQuery, build_generation_rag_queries,
    fuse_generation_rag_results, generation_rag_context_from_results,
    generation_rag_search_data_types, previous_chapter_range, validate_generation_query_vectors,
};
pub use service::ChapterGenerationService;

pub struct ChapterGenerateStreamInput<'a, F>
where
    F: FnMut(&str, u32),
{
    pub project_id: i64,
    pub chapter_number: i32,
    pub chapter_outline: &'a str,
    pub context_before: Option<&'a str>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, serde_json::Value>,
    pub workflow_run_id: i64,
    pub cancel_token: Option<CancellationToken>,
    pub on_chunk: F,
}

impl OrchestratorContext {
    pub async fn generate_chapter_stream<F>(
        &self,
        input: ChapterGenerateStreamInput<'_, F>,
    ) -> Result<chapter::Model, AppError>
    where
        F: FnMut(&str, u32),
    {
        let ChapterGenerateStreamInput {
            project_id,
            chapter_number,
            chapter_outline,
            context_before,
            prompt_renderer,
            variables,
            workflow_run_id,
            cancel_token,
            on_chunk,
        } = input;
        self.chapter
            .generate_stream(service::ChapterGenerationStreamInput {
                project_id,
                chapter_number,
                chapter_outline,
                context_before,
                llm: self.llm.as_ref(),
                prompt_renderer,
                variables,
                workflow_run_id,
                cancel_token,
                on_chunk,
            })
            .await
    }
}
