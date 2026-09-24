pub use a4n_video_provider_contract::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, ProviderImageInputTransport,
    SEEDANCE2_VIDEO_BACKEND, VIDEO_PROVIDER_CAPABILITIES, VideoGenerationResult,
    VideoJobDeleteOutcome, VideoJobState, VideoJobStatus, VideoProvider, VideoProviderCapabilities,
    validate_video_provider_duration, validate_video_provider_idempotency_key,
    video_provider_capabilities,
};
pub use a4n_video_sidecar_contract::VideoMediaSource;
pub type VideoGenerationRequest = a4n_video_sidecar_contract::VideoGenerationInput;
pub type VideoMediaInput = a4n_video_sidecar_contract::VideoMediaInput<VideoMediaSource>;

mod assembly;
mod config;
mod import;
mod lifecycle;
mod prompt;
mod provider_image;
mod snapshot;
mod tail_frame;
mod workflow;
pub use assembly::assembly_media_properties;
pub use config::{
    VIDEO_DURATION_MODE_FIXED, VIDEO_DURATION_MODE_RANGE, VideoConfigParams,
    VideoDurationCapability, duration_capability_for_params, validate_video_config_params,
    validate_video_duration_for_params_secs, validate_video_max_concurrent,
};
pub use import::{
    canvas_frame_timestamp, import_video_extension, imported_chapter_video_metadata,
    trimmed_video_metadata, validate_upload_video_mime, validate_video_trim_range,
};
pub use lifecycle::ensure_provider_generation_run_updated;
pub use prompt::{append_video_request_prompt_length_context, video_request_prompt_length_context};
pub use provider_image::{
    encode_query_component, is_provider_supported_image_content_type,
    provider_image_input_transport_for_backend, truncate_error_body, truncate_url_for_error,
};
pub use snapshot::provider_request_snapshot_value;
pub use tail_frame::{
    completed_provider_metadata, is_first_frame_chain_workflow, required_video_chapter_id,
    should_capture_video_tail_frame_fields, tail_frame_metadata_belongs_to_video,
    video_shot_number,
};
pub use workflow::is_canvas_video_record_fields;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_dimensions_must_be_positive() {
        let request = VideoGenerationRequest {
            prompt: "test".into(),
            negative_prompt: None,
            width: 0,
            height: 720,
            duration_secs: 5,
            fps: 24,
            model: "model".into(),
            media: VideoMediaInput::None,
        };
        assert!(request.validate_dimensions().is_err());
    }
}
