mod api;
mod contracts;
mod execution;
#[cfg(feature = "embedded-media-providers")]
mod preview;
mod request;
mod rules;

pub use self::api::{
    generate_reference_media_video, preview_reference_media_video_request,
    preview_video_provider_request_by_video_id, reference_media_video_generation_task_scope,
};
pub(crate) use self::rules::reference_media_dimensions_for_aspect_ratio;

#[cfg(all(test, feature = "embedded-media-providers"))]
pub(in crate::actions::video_generation) use self::contracts::ReferenceMediaProviderRequestPreviewContext;
#[cfg(all(test, feature = "embedded-media-providers"))]
pub(in crate::actions::video_generation) use self::preview::reference_media_provider_request_preview;
#[cfg(test)]
pub(in crate::actions::video_generation) use self::rules::{
    ensure_reference_media_tokens, reference_media_prompt, reference_media_urls,
};
