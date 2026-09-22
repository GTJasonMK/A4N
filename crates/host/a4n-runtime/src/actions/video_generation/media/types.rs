pub(super) use a4n_video_sidecar_contract::{
    MAX_VIDEO_IMAGE_BYTES as MAX_REFERENCE_IMAGE_BYTES,
    MAX_VIDEO_IMAGE_TOTAL_BYTES as MAX_REFERENCE_IMAGE_TOTAL_BYTES,
};
pub(super) const MEDIA_TOKEN_TTL_ENV: &str = "A4N_VIDEO_MEDIA_TOKEN_TTL_MINUTES";
pub(super) const REMOTE_TEMP_UPLOAD_URL_ENV: &str = "A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL";
pub(super) const REMOTE_TEMP_UPLOAD_TOKEN_ENV: &str = "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN";
pub(super) const PROVIDER_IMAGE_URL_PROBE_TIMEOUT_SECS: u64 = 20;

pub(crate) use a4n_video::ProviderImageInputTransport;
