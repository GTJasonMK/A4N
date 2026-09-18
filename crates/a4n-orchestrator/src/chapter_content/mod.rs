//! Chapter content 公共纯规则。
//!
//! 数据库行为与后处理编排由 `context/chapter_content.rs` 的私有
//! `ChapterContentApplication` 持有。DB 输入 DTO 不再经本模块穿透导出。

mod application;
mod rules;
mod sync;

pub(crate) use sync::CHAPTER_CONTENT_SYNC_RAG_STEP;
pub(crate) use sync::CHAPTER_CONTENT_SYNC_WORKFLOW_KIND;
pub(crate) use sync::chapter_content_sync_run_input_matches;
pub(crate) use sync::{
    CHAPTER_CONTENT_SYNC_CLEANUP_STEP, chapter_content_sync_checkpoint_artifacts,
    chapter_content_sync_checkpoint_input, chapter_content_sync_workflow_business_key,
    chapter_content_sync_workflow_run_input,
};

pub(crate) use rules::chapter_content_import_error_to_app_error;
