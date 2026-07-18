use crate::AppState;
use crate::actions::access;
use crate::actions::image_records::{IMPORTED_IMAGE_MODEL, read_uploaded_image_data};
use crate::actions::portrait_records::portrait_project_id_for_user;
use crate::actions::request_preview::ModelRequestPreview;
use crate::actions::template_variables::parse_template_variables;
use crate::file_utils::remove_data_file_in_data_dir;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::character::protagonist_profile;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_orchestrator::character::CharacterPortraitCreateInput;
use a4n_orchestrator::generation::generation_checkpoint_fingerprint_sha256;
use a4n_orchestrator::portrait::{
    PortraitPromptRequestInput, PortraitPromptSpec, design_brief::character_sheet_seed_instruction,
};
use a4n_orchestrator::workflow::{
    CompletedWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING,
};
use tokio_util::sync::CancellationToken;

mod generation;
mod image_pipeline;
mod preview;
mod prompt;
mod types;
mod upload;
mod workflow;

pub use generation::{
    generate_age_variant, generate_age_variant_task_scope, generate_missing_portraits,
    generate_missing_portraits_task_scope, generate_portrait, generate_portrait_task_scope,
    regenerate_portrait, regenerate_portrait_task_scope,
};
pub(crate) use image_pipeline::to_summary;
pub use preview::{
    preview_generate_age_variant_request, preview_generate_missing_portraits_request,
    preview_generate_portrait_request, preview_regenerate_portrait_request,
};
pub use prompt::{generate_portrait_prompt, preview_generate_portrait_prompt_request};
pub use types::*;
pub use upload::upload_portrait;

use image_pipeline::{
    character_portrait_by_id, persist_generated_portrait, run_portrait_image_generation,
};
use preview::{chat_preview_step, portrait_preview};
#[cfg(test)]
use prompt::{
    DEFAULT_PORTRAIT_EXPRESSION, first_expression_or_default, optional_profile_text,
    portrait_profile_context,
};
use prompt::{build_generate_portrait_prompt_request, profile_portrait_seed_prompt};
use workflow::{
    finalize_portrait_workflow, load_completed_portrait_final_checkpoint,
    missing_portrait_item_fingerprint,
};

#[cfg(test)]
mod tests;
