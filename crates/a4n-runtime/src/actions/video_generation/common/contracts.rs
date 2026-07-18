use a4n_core::models::ai::video_gen_config;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_video::VideoGenerationRequest;

use super::super::BoundaryFrameImageIds;

#[derive(Debug, Clone, Copy)]
pub(in crate::actions::video_generation) struct ProviderPromptImageBinding {
    pub(in crate::actions::video_generation) has_first_frame_image: bool,
    pub(in crate::actions::video_generation) has_last_frame_image: bool,
    pub(in crate::actions::video_generation) reference_image_count: usize,
    pub(in crate::actions::video_generation) has_storyboard_reference_image: bool,
}

#[derive(Clone)]
pub(in crate::actions::video_generation) struct ChapterVideoProviderRequestContext {
    pub(in crate::actions::video_generation) prompt: chapter_video_prompt::Model,
    pub(in crate::actions::video_generation) cfg: video_gen_config::Model,
    pub(in crate::actions::video_generation) request: VideoGenerationRequest,
    pub(in crate::actions::video_generation) prompt_reference_image_ids: Option<serde_json::Value>,
    pub(in crate::actions::video_generation) provider_reference_image_ids:
        Option<serde_json::Value>,
    pub(in crate::actions::video_generation) prompt_source: Option<serde_json::Value>,
    pub(in crate::actions::video_generation) workflow_mode: String,
    pub(in crate::actions::video_generation) source_prompt_type: Option<String>,
    pub(in crate::actions::video_generation) video_execution_identity: serde_json::Value,
    pub(in crate::actions::video_generation) boundary_frame_ids: BoundaryFrameImageIds,
    pub(in crate::actions::video_generation) prompt_reference_image_count: usize,
    pub(in crate::actions::video_generation) provider_reference_image_count: usize,
    pub(in crate::actions::video_generation) first_frame_image_url_available: bool,
    pub(in crate::actions::video_generation) last_frame_image_url_available: bool,
    pub(in crate::actions::video_generation) provider_input_note: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::actions::video_generation) enum VideoRequestImageMode {
    ProviderImageInputs,
    PreviewPlaceholders,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::actions::video_generation) enum VideoProviderRequestPurpose {
    Generate,
    Preview,
}
