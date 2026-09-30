use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PRICE_MICRO_SCALE: i64 = 1_000_000;
pub const MILLION: u64 = 1_000_000;
pub const SYSTEM_DEFAULT_PROVIDER_CONFIG_ID: i64 = -1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PricingPlan {
    pub currency: String,
    pub llm_input_per_million: Option<String>,
    pub llm_output_per_million: Option<String>,
    pub llm_cache_read_per_million: Option<String>,
    pub llm_cache_write_per_million: Option<String>,
    pub embedding_input_per_million: Option<String>,
    pub image_per_generation: Option<String>,
    pub video_per_second: Option<String>,
}

impl Default for PricingPlan {
    fn default() -> Self {
        Self {
            currency: "USD".to_string(),
            llm_input_per_million: None,
            llm_output_per_million: None,
            llm_cache_read_per_million: None,
            llm_cache_write_per_million: None,
            embedding_input_per_million: None,
            image_per_generation: None,
            video_per_second: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelCapability {
    Llm,
    Embedding,
    Image,
    Video,
}

impl ModelCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Embedding => "embedding",
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ModelUsageMeter {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub image_count: Option<u64>,
    pub duration_seconds: Option<String>,
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CostConfidence {
    Exact,
    Estimated,
    Partial,
    Unpriced,
    Unknown,
}

impl CostConfidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Estimated => "estimated",
            Self::Partial => "partial",
            Self::Unpriced => "unpriced",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostEstimate {
    pub amount_micros: Option<i64>,
    pub confidence: CostConfidence,
}
