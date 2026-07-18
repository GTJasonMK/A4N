use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::chapter_generation::ChapterGenerationWordCountRange;
use serde::Deserialize;
use std::collections::HashMap;

pub type ChapterGenerationProgress<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);
pub type ChapterGenerationChunk<'a> = &'a mut (dyn FnMut(&str, u32) -> Result<(), AppError> + Send);

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterStreamInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub task_id: Option<String>,
    pub writing_notes: Option<String>,
    pub min_word_count: Option<i32>,
    pub max_word_count: Option<i32>,
    pub client_rag_context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewChapterPromptInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub writing_notes: Option<String>,
    pub min_word_count: Option<i32>,
    pub max_word_count: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinishChapterGenerationRagContextInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub writing_notes: Option<String>,
    pub embedding_identity: EmbeddingModelIdentity,
    pub query_vectors: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterGenerationTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub(super) struct PreparedChapterGeneration {
    pub(super) chapter_title: String,
    pub(super) outline_text: String,
    pub(super) context_before: Option<String>,
    pub(super) word_count_range: ChapterGenerationWordCountRange,
    pub(super) use_rag: bool,
    pub(super) variables: HashMap<String, serde_json::Value>,
}
