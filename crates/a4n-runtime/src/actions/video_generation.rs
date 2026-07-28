#[cfg(test)]
use self::provider_request::provider_request_snapshot_value;
use self::provider_request::{
    model_request_preview_from_provider_snapshot, validate_video_provider_request_snapshot_owner,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::run_queued_request_with_cancel;
pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
use crate::actions::request_preview::{model_request_preview, provider_request_step};
use crate::actions::video_prompts::resolve_project_chapter_video_source;
#[cfg(test)]
use a4n_core::models::ai::generated_video;
#[cfg(test)]
use serde_json::json;

mod canvas;
mod chapter;
mod common;
mod config_state;
mod contracts;
mod generation_task;
mod job_lifecycle;
pub(crate) mod media;
mod poll;
mod provider_host;
mod provider_request;
mod reference_media;

pub use self::canvas::preview_canvas_video_request;
pub use self::chapter::preview_chapter_video_request;
pub(in crate::actions) use self::common::video_sidecar_root_project_id;
pub(in crate::actions) use self::config_state::resolve_video_config;
pub(crate) use self::contracts::video_to_summary;
pub use self::contracts::{
    GenerateCanvasVideoInput, GenerateChapterVideoInput, GenerateReferenceMediaVideoInput,
    GeneratedVideoSummary, PreviewChapterVideoPromptOverrideInput, ReferenceMediaVideoInput,
};
pub(crate) use self::media::{RemoteTempMediaDeleteOutcome, delete_remote_temp_media_artifact};
pub use self::poll::poll_video_job;
pub(in crate::actions) use self::provider_host::RuntimeVideoProviderHostPort;
pub use self::reference_media::{
    preview_reference_media_video_request, preview_video_provider_request_by_video_id,
};
pub(crate) use a4n_orchestrator::video::VideoGenerationProgress;
pub use a4n_orchestrator::video::has_active_provider_job;
pub use generation_task::start_video_generation_typed;
pub(in crate::actions) use job_lifecycle::cleanup_saved_video_outputs;

use self::common::*;

#[cfg(test)]
mod tests;
