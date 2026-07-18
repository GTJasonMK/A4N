#[cfg(not(feature = "embedded-media-providers"))]
mod disabled;
#[cfg(feature = "embedded-media-providers")]
mod embedded;

#[cfg(not(feature = "embedded-media-providers"))]
pub(in crate::actions::video_generation) use disabled::video_provider_preview_payload;
#[cfg(feature = "embedded-media-providers")]
pub(in crate::actions::video_generation) use embedded::video_provider_preview_payload;
