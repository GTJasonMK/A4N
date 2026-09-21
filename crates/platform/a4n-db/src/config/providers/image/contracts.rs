use serde_json::Value;

pub struct ImageConfigRecordInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes_json: Value,
    pub pricing_json: Value,
}
