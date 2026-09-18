//! In-process boundary used by dynamic LLM, embedding, image, and video providers.
//!
//! The contract deliberately exposes provider configuration and task leases,
//! not ORM models or the orchestrator context. The orchestrator supplies the
//! implementation; runtime provider code only depends on this crate.

use a4n_domain::cost::PricingPlan;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct LlmProviderConfig {
    pub id: i64,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub extra_body_json: Option<Value>,
    pub agent_protocol: String,
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingProviderConfig {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageProviderConfig {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes_json: Value,
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoProviderConfig {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub is_default: bool,
    pub pricing: PricingPlan,
}
