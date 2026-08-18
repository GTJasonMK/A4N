use serde::{Deserialize, Serialize};

pub(crate) use a4n_domain::DiscoveredModel;
pub use a4n_domain::ModelDiscoveryResult;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDiscoveryRequest {
    pub provider: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
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
        .err()
        .expect("unknown discovery fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_model`"));
    }
}
