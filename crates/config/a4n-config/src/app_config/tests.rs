use super::*;

#[test]
fn app_config_does_not_default_to_a_database_url() {
    assert_eq!(AppConfig::default().database_url, "");
}

#[test]
fn app_config_defaults_to_offline_embedding() {
    let embedding = AppConfig::default().embedding;
    assert_eq!(embedding.backend, "local_hash");
    assert_eq!(embedding.model, "local-sha256-hash");
    assert_eq!(embedding.dimension, 384);
    assert!(embedding.base_url.is_empty());
    assert!(embedding.api_key.is_none());
}
