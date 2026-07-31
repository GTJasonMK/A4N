use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::chapter::chapter_outline;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::outline::OutlineRequestPreviewStep;

mod access;
mod batch;
mod chapters;
mod generation_task;
mod parts;
mod preview;
mod types;

pub use batch::{
    preview_apply_beat_mapping_request, preview_batch_generate_part_outlines_request,
    preview_generate_chapter_outlines_by_count_request,
};
pub use chapters::{
    delete_latest_chapter_outlines, get_chapter_outlines_pacing, list_chapter_outlines,
    preview_generate_part_chapter_outlines_request, preview_regenerate_chapter_outline_request,
    update_chapter_outline,
};
pub use generation_task::start_outline_generation_typed;
pub use parts::{
    delete_latest_parts, get_generation_progress, list_parts,
    preview_continue_part_outlines_request, preview_generate_parts_request,
    preview_regenerate_last_part_request, preview_regenerate_part_outline_request,
    preview_regenerate_parts_request,
};
pub use types::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlineSummary,
    ContinuePartOutlinesInput, GenerateChapterOutlinesByCountInput,
    GeneratePartChapterOutlinesInput, GeneratePartsInput, GenerationProgressSummary,
    PartOutlineSummary, RegenerateChapterOutlineInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput, UpdateChapterOutlineInput,
};

#[cfg(test)]
use access::ensure_chapter_outline_belongs_to_project;
use access::{ensure_chapter_outline_access, ensure_part_access};
#[cfg(test)]
use chapters::build_chapter_outlines_pacing;
use preview::{outline_preview, single_request_preview};
use types::chapter_outline_summary;

#[cfg(test)]
mod tests;
use crate::AppState;
