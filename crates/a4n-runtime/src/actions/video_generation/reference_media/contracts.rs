use super::super::*;

#[derive(Debug)]
pub(super) struct ReferenceMediaProviderRequestContext {
    pub(super) cfg: video_gen_config::Model,
    pub(super) request: VideoGenerationRequest,
    pub(super) provider_input_note: Option<String>,
}

#[cfg(feature = "embedded-media-providers")]
pub(in crate::actions::video_generation) struct ReferenceMediaProviderRequestPreviewContext {
    pub(in crate::actions::video_generation) request: VideoGenerationRequest,
    pub(in crate::actions::video_generation) config_id: Option<i64>,
    pub(in crate::actions::video_generation) provider_backend: Option<String>,
    pub(in crate::actions::video_generation) provider_input_note: Option<String>,
}
