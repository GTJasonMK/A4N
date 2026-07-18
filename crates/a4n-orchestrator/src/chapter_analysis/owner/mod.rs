//! 章节结构化分析
//!
//! 该服务把已确定的章节正文转为稳定的 JSON 产物，供摘要、角色状态、伏笔、
//! RAG 和后续多媒体流程复用。

mod assembly;
mod extract;
mod materialize;
mod normalization;
mod prompt;
mod requests;
mod validation;

#[cfg(test)]
mod tests;

pub use assembly::assemble_staged_chapter_analysis;
pub use extract::chapter_analysis_storage_projection;
pub use materialize::materialize_analysis_source_spans;
pub use normalization::normalize_analysis;
pub use requests::{
    build_chapter_analysis_preview_requests, build_chapter_character_state_request,
    build_chapter_fact_extraction_request, build_chapter_foreshadowing_events_request,
    build_chapter_summary_synthesis_request, chapter_analysis_root_field_progress,
    json_value_shape_summary,
};
pub use validation::{
    validate_chapter_character_state, validate_chapter_fact_extraction,
    validate_chapter_foreshadowing_events, validate_chapter_summary_synthesis,
};

#[cfg(test)]
use a4n_core::error::AppError;
#[cfg(test)]
use serde_json::Value;
