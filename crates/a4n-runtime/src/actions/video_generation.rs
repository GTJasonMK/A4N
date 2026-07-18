use self::boundary_frames::{
    BoundaryFrameImageIds, boundary_frame_image_ids_for_prompt, boundary_frame_numbers_for_prompt,
};
use self::config_state::{
    ensure_created_video_capacity, ensure_video_capacity, resolve_video_config,
};
use self::first_frame_chain::first_frame_chain_start_image_id_for_prompt;
use self::media::{
    ProviderImageInputTransport, reference_image_preview_placeholder_by_id,
    reference_image_preview_placeholders, reference_image_provider_input_by_id,
    reference_image_provider_inputs, remote_temp_upload_configured,
};
#[cfg(all(test, feature = "embedded-media-providers"))]
use self::provider_request::chapter_video_provider_request_snapshot;
use self::provider_request::{
    CliproxyGrokReferenceMode, append_preview_duration_note, append_provider_input_note,
    build_chapter_video_provider_request, chapter_video_provider_request_snapshot_for_state,
    cliproxy_grok_reference_mode, ensure_provider_generation_workflow,
    model_request_preview_from_provider_snapshot, provider_request_snapshot_value,
    validate_video_provider_request_snapshot_owner, video_provider_preview_payload,
};
#[cfg(test)]
use self::provider_request::{
    apply_chapter_video_prompt_preview_override, boundary_frame_missing_image_message,
    build_video_generation_provider_preview_prompt_with_binding,
    build_video_generation_provider_prompt_with_binding, enforce_cliproxy_video_media_contract,
    enforce_provider_image_binding_contract, enforce_provider_prompt_length_contract,
    ensure_chapter_video_prompt_preview_metadata, ensure_chapter_video_prompt_preview_texts,
    ensure_newapi_first_frame_image_supported, linked_video_preview_config_id,
    prompt_is_first_frame_chain, provider_default_image_input_transport_with_remote,
    provider_image_input_transport_with_override, provider_specific_negative_prompt,
    reference_ids_with_boundary_frame_images,
};
use self::storyboard_frames::storyboard_image_id_for_prompt;
use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{
    run_queued_request_with_cancel, run_queued_request_with_cancel_and_workflow,
};
pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
use crate::actions::request_preview::{model_request_preview, provider_request_step};
use crate::actions::video_prompt_text::{
    MIN_PROVIDER_MOTION_PROMPT_CHARS, MIN_PROVIDER_VISUAL_PROMPT_CHARS,
    ProviderPromptTextViolation, provider_prompt_text_violation, validate_provider_model_text,
};
use crate::actions::video_prompts::{
    ensure_prompt_matches_current_chapter, resolve_project_chapter_video_source,
};
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::ai::generated_video;
use a4n_core::models::ai::video_gen_config;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::CreateGeneratedVideoRecordDraftInput as CreateVideoRecord;
use a4n_orchestrator::video::{PromptOutputLanguage, validate_video_duration_for_config_secs};
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_video::{VideoGenerationRequest, VideoReferenceMediaRequest};
#[cfg(feature = "embedded-media-providers")]
use a4n_video_providers::{
    ChatCompletionsVideoProvider, CliProxyOpenAiVideoProvider, GeminiVeoVideoProvider,
    HttpVideoProvider, NewApiVideoProvider, Seedance2VideoProvider,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value, json};
use tokio_util::sync::CancellationToken;

mod boundary_frames;
mod canvas;
mod chapter;
mod common;
mod config_state;
mod contracts;
mod first_frame_chain;
mod job_lifecycle;
pub(crate) mod media;
mod poll;
mod prompt_chain;
mod provider_request;
mod reference_media;
mod results;
mod storyboard_frames;

pub use self::canvas::{generate_canvas_video, preview_canvas_video_request};
pub use self::chapter::{
    chapter_video_generation_task_scope, chapter_videos_batch_generation_task_scope,
    generate_chapter_video, generate_chapter_videos_batch, preview_chapter_video_request,
};
pub use self::common::has_active_provider_job;
pub use self::contracts::{
    GenerateCanvasVideoInput, GenerateChapterVideoInput, GenerateChapterVideosBatchInput,
    GenerateChapterVideosBatchResult, GenerateReferenceMediaVideoInput, GeneratedVideoSummary,
    PreviewChapterVideoPromptOverrideInput, ReferenceMediaVideoInput,
    VideoGenerationProgressCallback, VideoGenerationTaskScope,
};
pub(crate) use self::contracts::{VideoGenerationProgress, video_to_summary};
pub use self::poll::poll_video_job;
pub(crate) use self::reference_media::reference_media_dimensions_for_aspect_ratio;
pub use self::reference_media::{
    generate_reference_media_video, preview_reference_media_video_request,
    preview_video_provider_request_by_video_id, reference_media_video_generation_task_scope,
};

#[cfg(test)]
use self::canvas::canvas_video_provider_image_binding;
use self::chapter::{
    append_video_request_prompt_length_context, video_generation_task_scope,
    video_request_prompt_length_context,
};
use self::common::*;
#[cfg(all(test, feature = "embedded-media-providers"))]
use self::reference_media::{
    ReferenceMediaProviderRequestPreviewContext, reference_media_provider_request_preview,
};
#[cfg(test)]
use self::reference_media::{
    ensure_reference_media_tokens, reference_media_prompt, reference_media_urls,
};
use self::results::{
    VideoProviderResultHandling, ensure_provider_generation_run_updated,
    fail_video_generation_runs, handle_canvas_video_provider_result, handle_video_provider_result,
    mark_video_generation_failed, save_completed_canvas_video_result,
    started_video_generation_run_input,
};

#[cfg(test)]
mod tests;
