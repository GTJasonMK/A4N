use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(in crate::routes) struct TempMediaUploadRequest {
    pub(super) file_name: String,
    pub(super) mime_type: String,
    pub(super) data_base64: String,
}

#[derive(Debug, Serialize)]
pub(in crate::routes) struct TempMediaUploadResponse {
    pub(super) url: String,
    pub(super) expires_at: DateTime<Utc>,
    pub(super) size_bytes: usize,
}
