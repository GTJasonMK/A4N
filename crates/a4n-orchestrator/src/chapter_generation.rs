mod application;
mod context_before;
mod lifecycle;
mod preparation;
pub(crate) mod rag_context;

#[path = "chapter_generation/service/mod.rs"]
pub mod service;

pub use application::{
    ChapterGenerationApplicationContext, ChapterGenerationRequestPreview,
    PreparedGenerationRagEmbeddingRequest, PreviewChapterGenerationInput,
};
pub(crate) use lifecycle::StartChapterGenerationWorkflowInput;
pub use preparation::{
    ChapterGenerationWordCountRange, compact_json_text, generation_chapter_title,
    is_meaningful_json_value, merge_generation_outline_text, non_empty_trimmed,
    outline_character_names, resolve_generation_word_count_range, truncate_text_chars,
    upsert_chapter_generation_variables,
};
pub use rag_context::{
    GenerationRagContext, GenerationRagQueries, GenerationRagStatistics,
    PreparedGenerationRagSearch, WeightedGenerationQuery, build_generation_rag_queries,
    fuse_generation_rag_results, generation_rag_context_from_results,
    generation_rag_query_fingerprint, generation_rag_search_data_types, previous_chapter_range,
    validate_generation_query_vectors, validate_generation_rag_query_fingerprint,
};
pub use service::ChapterGenerationService;
