pub(super) use super::mapping::embedding_config_summary;
pub use a4n_api_contract::config::{
    CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput,
};

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
