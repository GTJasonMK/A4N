use crate::AppState;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::{
    outline::OutlineRequestPreviewStep,
    workflow::{
        UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
        WORKFLOW_STATUS_RUNNING,
    },
};
use serde_json::Value;

mod access;
mod batch;
mod chapters;
mod parts;
mod preview;
mod types;
mod workflow;

pub use access::{
    apply_beat_mapping_task_scope, batch_generate_part_outlines_task_scope,
    continue_part_outlines_task_scope, generate_chapter_outlines_by_count_task_scope,
    generate_part_chapter_outlines_task_scope, generate_parts_task_scope,
    regenerate_chapter_outline_task_scope, regenerate_last_part_task_scope,
    regenerate_part_outline_task_scope, regenerate_parts_task_scope,
};
pub use batch::{
    apply_beat_mapping, batch_generate_part_outlines, generate_chapter_outlines_by_count,
    preview_apply_beat_mapping_request, preview_batch_generate_part_outlines_request,
    preview_generate_chapter_outlines_by_count_request,
};
pub use chapters::{
    delete_latest_chapter_outlines, generate_part_chapter_outlines, get_chapter_outlines_pacing,
    list_chapter_outlines, preview_generate_part_chapter_outlines_request,
    preview_regenerate_chapter_outline_request, regenerate_chapter_outline, update_chapter_outline,
};
pub use parts::{
    continue_part_outlines, delete_latest_parts, generate_parts, get_generation_progress,
    list_parts, preview_continue_part_outlines_request, preview_generate_parts_request,
    preview_regenerate_last_part_request, preview_regenerate_part_outline_request,
    preview_regenerate_parts_request, regenerate_last_part, regenerate_part_outline,
    regenerate_parts,
};
pub use types::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlineSummary,
    ContinuePartOutlinesInput, GenerateChapterOutlinesByCountInput,
    GeneratePartChapterOutlinesInput, GeneratePartsInput, GenerationProgressSummary,
    OutlineTaskScope, PartOutlineSummary, RegenerateChapterOutlineInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput, UpdateChapterOutlineInput,
};

use access::{
    advance_status, batch_parts_progress_total, collect_unique_parts,
    ensure_chapter_outline_access, ensure_part_access, part_chapter_count, progress_total, report,
    required_outline_pacing_score,
};
#[cfg(test)]
use access::{ensure_chapter_outline_belongs_to_project, order_parts_for_batch_generation};
#[cfg(test)]
use chapters::build_chapter_outlines_pacing;
#[cfg(test)]
use parts::completed_part_count;
use preview::{outline_preview, single_request_preview};
use types::chapter_outline_summary;
use workflow::{
    finish_outline_postprocess, start_outline_workflow_run, unwrap_outline_workflow_result,
};

pub type OutlineProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[cfg(test)]
mod tests;
