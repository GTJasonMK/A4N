mod application;
mod context_before;
mod lifecycle;
mod preparation;
pub(crate) mod rag_context;

#[path = "chapter_generation/service/mod.rs"]
pub mod service;

pub use application::{
    ChapterGenerationApplicationContext, ChapterGenerationRequestPreview,
};
pub(crate) use lifecycle::StartChapterGenerationWorkflowInput;
pub use preparation::{
    ChapterGenerationWordCountRange, compact_json_text, generation_chapter_title,
    is_meaningful_json_value, merge_generation_outline_text, non_empty_trimmed,
    outline_character_names, resolve_generation_word_count_range, truncate_text_chars,
    upsert_chapter_generation_variables,
};
pub(crate) use rag_context::{
    PreparedGenerationRagSearch, generation_rag_query_fingerprint,
    validate_generation_rag_query_fingerprint,
};
pub use service::ChapterGenerationService;
