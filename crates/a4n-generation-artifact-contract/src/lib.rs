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
