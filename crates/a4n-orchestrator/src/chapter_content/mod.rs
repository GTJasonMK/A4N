//! Chapter content 公共纯规则。
//!
//! 数据库行为与后处理编排由 `context/chapter_content.rs` 的私有
//! `ChapterContentApplication` 持有。DB 输入 DTO 不再经本模块穿透导出。

mod application;
mod rules;
mod sync;

pub use application::ChapterDerivedCleanupHostPort;
pub(crate) use sync::{
    CHAPTER_CONTENT_SYNC_CLEANUP_STEP, chapter_content_sync_checkpoint_artifacts,
    chapter_content_sync_checkpoint_input, chapter_content_sync_workflow_business_key,
    chapter_content_sync_workflow_run_input,
};
pub use sync::{
    CHAPTER_CONTENT_SYNC_RAG_STEP, CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
    chapter_content_sync_run_input_matches,
};

pub(crate) use rules::{
    chapter_content_edit_error_to_app_error, chapter_content_import_error_to_app_error,
};
pub use rules::{
    ensure_chapter_content_not_empty, ensure_chapter_not_generating, ensure_import_chapter_number,
    ensure_text_project_kind,
};

/// 章节后处理结果值对象。
#[derive(Debug, Clone)]
pub struct PostProcessResult {
    pub style_tags: Vec<String>,
    pub char_count: usize,
    pub chinese_word_count: usize,
    pub paragraph_count: usize,
    pub dialogue_ratio: f64,
    pub quality_score: f64,
}

pub use a4n_db::chapter_content::{
    ChapterContentEditError, ChapterContentImportError, EditChapterContentInput,
    ImportChapterContentInput,
};
