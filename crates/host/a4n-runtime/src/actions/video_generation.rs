use crate::AppState;
use crate::actions::request_preview::ModelRequestPreview;
use crate::actions::request_preview::{model_request_preview, provider_request_step};
mod canvas;
mod chapter;
mod common;
mod config;
mod config_state;
mod generation_task;
mod job_lifecycle;
pub(crate) mod media;
mod poll;
mod preview;
mod provider_host;
mod provider_request;
mod reference_media;

pub(crate) use self::canvas::preview_canvas_video_request;
pub use self::chapter::preview_chapter_video_request;
pub(in crate::actions) use self::common::video_sidecar_root_project_id;
pub(in crate::actions) use self::config_state::resolve_video_config;
pub(crate) use self::media::{RemoteTempMediaDeleteOutcome, delete_remote_temp_media_artifact};
pub use self::poll::poll_video_job;
pub(in crate::actions) use self::provider_host::RuntimeVideoProviderHostPort;
pub use self::reference_media::{
    preview_reference_media_video_request, preview_video_provider_request_by_video_id,
};
pub use a4n_api_contract::video::{
    GenerateChapterVideoInput, GenerateReferenceMediaVideoInput, GeneratedVideoSummary,
    PreviewChapterVideoPromptOverrideInput,
};
pub use a4n_canvas_contract::GenerateCanvasVideoInput;
pub use generation_task::start_video_generation_typed;
pub(in crate::actions) use job_lifecycle::cleanup_saved_video_outputs;

#[cfg(test)]
mod tests;
