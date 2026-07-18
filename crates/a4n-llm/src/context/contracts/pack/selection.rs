use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPackRef {
    pub pack_kind: String,
    pub content_sha256: String,
    pub compression_level: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPackRequirement {
    pub pack_kind: String,
    pub required: bool,
    pub stable_prefix: bool,
    pub variants: Vec<LlmContextPackRef>,
}
