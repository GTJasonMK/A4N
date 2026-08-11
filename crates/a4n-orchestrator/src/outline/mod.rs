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
mod parts;
mod preview;
mod rules;
mod workflow_rules;

pub use application::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
    GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
    OutlineGenerationProgressSummary, RegenerateChapterOutlineInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput,
};
pub use contracts::{
    BeatMappingApplyInput, ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, OutlineRequestPreviewStep, PartOutlineContinuationInput,
    PartOutlineGenerationInput, PartOutlineRegenerationInput, ProgressReporter,
};
pub(in crate::outline) use llm_stream::{
    OutlineLlmStreamContext, execute_outline_llm_call, with_outline_llm_stream,
};
pub(in crate::outline) use preview::{
    blocked_outline_preview_step, ready_outline_preview_step, reused_outline_preview_step,
};
pub use rules::{
    batch_parts_progress_total, order_parts_for_batch_generation, part_chapter_count,
    progress_total, required_outline_pacing_score,
};
pub(in crate::outline) use rules::{
    ensure_response_not_truncated, report_progress, validate_existing_chapter_outline_for_request,
};
pub use workflow_rules::{outline_workflow_business_key, outline_workflow_run_input};

use crate::OrchestratorContext;

#[cfg(test)]
mod tests;

pub use chapter_contract::persistence::{
    insert_chapter_outline_from_json, outline_db_error as outline_db_error_from_chapter,
    sync_chapter_outlines_by_number_transaction, update_chapter_outline_from_json,
};
pub use context::{
    ChapterOutlineGenerationContext, PartOutlineGenerationContext,
    load_chapter_outline_generation_context, load_part_generation_context,
};
pub(crate) use context::{chapter_outline_context_json, outline_formal_materials_context_json};
pub use part_contract::model::PartOutlineDraft;
pub use part_contract::persistence::{
    insert_part_outline_from_json, sync_part_outlines_and_assign_chapters_transaction,
    update_part_outline_and_rebind_chapters_transaction, update_part_outline_from_json,
};
// outline_db_error: reexport under expected name
pub use chapter_contract::persistence::outline_db_error;
