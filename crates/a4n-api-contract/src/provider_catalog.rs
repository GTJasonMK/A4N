use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LlmProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub default_base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImageProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub requires_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub min_duration_secs: u32,
    pub max_duration_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingProviderDescriptor {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelProviderCatalog {
    pub llm: Vec<LlmProviderDescriptor>,
    pub image: Vec<ImageProviderDescriptor>,
    pub video: Vec<VideoProviderDescriptor>,
    pub embedding: Vec<EmbeddingProviderDescriptor>,
}
