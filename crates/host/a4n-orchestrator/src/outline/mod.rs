//! 大纲系统服务：分部大纲 + 章节大纲生成/管理
//!
//! 对应文档 04-大纲系统

mod application;
mod chapter_contract;
mod chapters;
mod context;
mod contracts;
mod generation;
mod llm_stream;
mod part_contract;
mod parts {
    mod continuation;
    mod contracts;
    mod generation {
        mod append;
        mod preview;
        mod replacement;
    }
    mod llm {
        mod execution;
        mod trace;
    }
    mod planning;
    mod queries;
    mod regeneration {
        mod execution;
        mod preparation;
        mod preview;
    }
    mod requests;

    #[cfg(test)]
    mod tests;
}
mod preview;
mod rules;
mod workflow_rules;

pub use a4n_outline_contract::{
    OutlineBatchPartGenerationPreviewApplicationPort, OutlineBeatMappingPreviewApplicationPort,
    OutlineChapterCountGenerationPreviewApplicationPort,
    OutlineChapterRegenerationPreviewApplicationPort,
    OutlinePartChapterGenerationPreviewApplicationPort,
    OutlinePartGenerationPreviewApplicationPort, OutlinePartMutationPreviewApplicationPort,
    OutlineRequestPreviewStep,
};
pub(in crate::outline) use contracts::{
    BeatMappingApplyInput, ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, PartOutlineContinuationInput, PartOutlineGenerationInput,
    PartOutlineRegenerationInput, ProgressReporter,
};
pub(in crate::outline) use llm_stream::{
    OutlineLlmStreamContext, execute_outline_llm_call, with_outline_llm_stream,
};
pub(in crate::outline) use preview::{ready_outline_preview_step, reused_outline_preview_step};
pub(in crate::outline) use rules::{
    batch_parts_progress_total, order_parts_for_batch_generation, part_chapter_count,
    progress_total,
};
pub(in crate::outline) use rules::{
    ensure_response_not_truncated, report_progress, validate_existing_chapter_outline_for_request,
};
pub(crate) use workflow_rules::outline_workflow_run_input;

use crate::OrchestratorContext;

#[cfg(test)]
mod tests;

pub(in crate::outline) use chapter_contract::outline_db_error;
pub(crate) use context::{chapter_outline_context_json, outline_formal_materials_context_json};
