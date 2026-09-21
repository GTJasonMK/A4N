pub(super) const MAX_REFERENCE_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
pub(super) const MAX_REFERENCE_IMAGE_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
pub(super) const MEDIA_TOKEN_TTL_ENV: &str = "A4N_VIDEO_MEDIA_TOKEN_TTL_MINUTES";
pub(super) const REMOTE_TEMP_UPLOAD_URL_ENV: &str = "A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL";
pub(super) const REMOTE_TEMP_UPLOAD_TOKEN_ENV: &str = "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN";
pub(super) const PROVIDER_IMAGE_URL_PROBE_TIMEOUT_SECS: u64 = 20;

pub(crate) use a4n_video::ProviderImageInputTransport;
