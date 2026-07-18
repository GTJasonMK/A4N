use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDiscoveryRequest {
    pub provider: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveredModel {
    pub id: String,
    pub name: Option<String>,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDiscoveryResult {
    pub models: Vec<DiscoveredModel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_request_rejects_unknown_fields() {
        let error = serde_json::from_value::<ModelDiscoveryRequest>(serde_json::json!({
            "provider": "openai",
            "base_url": "http://127.0.0.1:1",
            "legacy_model": "test"
        }))
        .expect_err("unknown discovery fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_model`"));
    }
}
