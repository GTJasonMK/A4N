use super::super::shared::deserialize_optional_string_update;
use serde::{Deserialize, Serialize};

pub(super) use super::mapping::{
    embedding_config_summary, embedding_create_record_input, embedding_update_record_input,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateEmbeddingInput {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateEmbeddingInput {
    pub name: Option<String>,
    pub backend: Option<String>,
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string_update")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: Option<String>,
    pub dimension: Option<i32>,
    pub batch_size: Option<i32>,
    pub local_model_path: Option<String>,
}

pub fn list_embedding_backends() -> Vec<String> {
    vec!["api".into(), "fastembed".into(), "local_hash".into()]
}

#[cfg(test)]
mod tests {
    use super::list_embedding_backends;

    #[test]
    fn list_embedding_backends_is_stable_contract() {
        assert_eq!(
            list_embedding_backends(),
            vec!["api", "fastembed", "local_hash"]
        );
    }
}
