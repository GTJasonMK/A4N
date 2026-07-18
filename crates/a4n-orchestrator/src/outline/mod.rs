//! 大纲系统服务：分部大纲 + 章节大纲生成/管理
//!
//! 对应文档 04-大纲系统

mod chapter_contract;
mod chapters;
mod context;
mod contracts;
mod db_facade;
mod part_contract;
mod parts;
mod preview;
mod rules;

pub use contracts::{
    BeatMappingApplyInput, ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, OutlineRequestPreviewStep, PartOutlineContinuationInput,
    PartOutlineGenerationInput, PartOutlineRegenerationInput, ProgressReporter,
};
pub(in crate::outline) use preview::{
    blocked_outline_preview_step, ready_outline_preview_step, reused_outline_preview_step,
};
pub(in crate::outline) use rules::{
    ensure_response_not_truncated, report_progress, validate_existing_chapter_outline_for_request,
};

use crate::OrchestratorContext;

#[cfg(test)]
mod tests;
