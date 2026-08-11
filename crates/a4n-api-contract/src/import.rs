use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_PROJECT_TEXT_UPLOAD_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectTextUploadInput {
    pub project_id: i64,
    pub file_name: String,
    pub data_base64: String,
}
