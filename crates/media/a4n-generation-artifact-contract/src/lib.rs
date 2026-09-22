//! Pure artifact references shared by generation workers and Host adapters.
//!
//! This leaf contract deliberately does not depend on the full service
//! protocol so media contracts can use artifact staging without inheriting
//! unrelated transport events and validation code.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const JS_SAFE_INTEGER_U64: u64 = 9_007_199_254_740_991;
const JS_SAFE_INTEGER_I64: i64 = 9_007_199_254_740_991;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_KEY_BYTES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub artifact_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
    pub location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub ownership: ArtifactOwnership,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactLocation {
    LocalPath(String),
    UploadUrl(String),
    DownloadUrl(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOwnership {
    Host,
    Sidecar,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationArtifactRef {
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub artifact_id: String,
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub artifact_kind: String,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub content_type: String,
    #[schemars(length(equal = 64))]
    pub sha256: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub byte_size: u64,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub revision: String,
    pub temporary: bool,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub expires_at_ms: Option<i64>,
}
