mod prelude {
    pub(super) use crate::actions::request_preview::ChatStatusPreviewStep;
    pub(super) use crate::actions::{access, app_error_message};
    pub(super) use crate::prompt_renderer::RuntimePromptRenderer;
    pub(super) use crate::state::AppState;
    pub(super) use a4n_core::chapter_analysis::validate_saved_chapter_analysis;
    pub(super) use a4n_core::error::AppError;
    pub(super) use a4n_core::models::chapter::chapter;
    pub(super) use a4n_orchestrator::workflow::{
        CompletedWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
        required_checkpoint_field, required_checkpoint_json,
    };
    pub(super) use serde::Deserialize;
    pub(super) use serde_json::{Value, json};
    pub(super) use tokio_util::sync::CancellationToken;
}

mod api;
mod checkpoint;
mod contracts;
mod execution;
mod outputs;
mod preview;
mod sync;

pub use api::{
    generate_chapter_analysis_with_progress, preflight_generate_chapter_analysis,
    preflight_resume_chapter_analysis_sync, preview_chapter_analysis_request,
    preview_resume_chapter_analysis_sync_request, resume_chapter_analysis_sync_with_progress,
    resume_chapter_analysis_workflow_run,
};
pub use contracts::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisProgress, GenerateChapterAnalysisInput,
};
pub use execution::{
    ensure_previous_chapter_analysis_complete, generate_and_sync_chapter_analysis_with_progress,
};
pub use sync::sync_project_state_after_chapter_analysis_with_progress;

use a4n_orchestrator::chapter_analysis::{
    chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
    chapter_analysis_workflow_business_key, ensure_chapter_analysis_character_sync_completed,
    require_completed_chapter_analysis_phase,
};
use checkpoint::{
    completed_chapter_analysis_phase, mark_chapter_analysis_current_step,
    preflight_chapter_analysis, save_completed_chapter_analysis_phase,
};
use contracts::{
    CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
    CHAPTER_ANALYSIS_STEP_DERIVED_SYNC, CHAPTER_ANALYSIS_STEP_FACTS,
    CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY, CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
    CHAPTER_ANALYSIS_STEP_SCENE_SYNC, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
};
use execution::load_chapter_by_id;
use outputs::sync_chapter_analysis_outputs;
#[cfg(test)]
use preview::map_subprogress;
use preview::{
    blocked_chapter_analysis_step, blocked_scene_profile_sync_chapter_analysis_steps,
    character_discovery_chapter_analysis_steps, ensure_chapter_not_generating,
    object_discovery_chapter_analysis_steps, report_chapter_analysis_progress,
    report_chapter_subprogress, scene_profile_sync_chapter_analysis_steps,
};

#[cfg(test)]
mod tests;
