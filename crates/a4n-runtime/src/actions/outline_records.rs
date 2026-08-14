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
    save_chapter_outline, update_chapter_outline,
};
pub use generation_task::start_outline_generation_typed;
pub use parts::{
    delete_latest_parts, get_generation_progress, list_parts,
    preview_continue_part_outlines_request, preview_generate_parts_request,
    preview_regenerate_last_part_request, preview_regenerate_part_outline_request,
    preview_regenerate_parts_request, save_part_outline,
};
pub use types::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlinePacing,
    ChapterOutlineSummary, ContinuePartOutlinesInput, DeleteLatestOutlineRecordsInput,
    GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
    OutlineGenerationProgressSummary, OutlineMutationResult, PartOutlineSummary,
    RegenerateChapterOutlineInput, RegenerateLastPartInput, RegeneratePartOutlineInput,
    RegeneratePartsInput, SaveChapterOutlineInput, SaveChapterOutlineResult, SavePartOutlineInput,
    SavePartOutlineResult, UpdateChapterOutlineInput, UpdateChapterOutlineResult,
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

pub async fn sync_outline_materials(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<OutlineMutationResult, AppError> {
    crate::actions::access::verify_project_editor(state, user_id, project_id).await?;
    Ok(OutlineMutationResult {
        warnings: outline_material_sync_warnings(state, project_id).await,
    })
}

pub(crate) async fn outline_material_sync_warnings(
    state: &AppState,
    project_id: i64,
) -> Vec<String> {
    crate::rag_ingest::sync_outline_rag_records(state, project_id)
        .await
        .err()
        .map(|error| {
            format!("大纲检索索引未完全更新：{error}。正式资料未受影响，可稍后在项目操作中重新同步")
        })
        .into_iter()
        .collect()
}

pub(in crate::actions::outline_records) fn project_outline_delete_input(
    input: DeleteLatestOutlineRecordsInput,
) -> a4n_orchestrator::project::DeleteLatestOutlineRecordsInput {
    a4n_orchestrator::project::DeleteLatestOutlineRecordsInput {
        project_id: input.project_id,
        targets: input
            .targets
            .into_iter()
            .map(|target| a4n_orchestrator::project::OutlineDeleteTarget {
                id: target.id,
                expected_updated_at: target.expected_updated_at,
            })
            .collect(),
    }
}
