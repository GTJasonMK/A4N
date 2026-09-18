use a4n_db::config::{
    LlmConfigRecordInput, ProviderConfigKind, create_llm_config_record, delete_llm_config_record,
    list_llm_config_records, set_default_provider_config,
};
use uuid::Uuid;

fn llm_input(name: String) -> LlmConfigRecordInput {
    LlmConfigRecordInput {
        name,
        provider: "openai".to_string(),
        base_url: "https://llm.example.test/v1".to_string(),
        api_key_encrypted: None,
        model: "test-model".to_string(),
        max_concurrent: 1,
        timeout_secs: 30,
        max_context_tokens: 8192,
        max_output_tokens: 2048,
        agent_protocol: "native_tool_calling".to_string(),
        extra_body_json: None,
        pricing_json: serde_json::json!({}),
    }
}

#[tokio::test]
async fn default_switch_is_atomic_and_keeps_one_default() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = Uuid::new_v4();
    let first = create_llm_config_record(&db, llm_input(format!("default-a-{suffix}")))
        .await
        .unwrap();
    let second = create_llm_config_record(&db, llm_input(format!("default-b-{suffix}")))
        .await
        .unwrap();

    set_default_provider_config(&db, ProviderConfigKind::Llm, first.id)
        .await
        .unwrap();
    assert!(
        set_default_provider_config(&db, ProviderConfigKind::Llm, i64::MAX)
            .await
            .is_err()
    );
    let rows = list_llm_config_records(&db).await.unwrap();
    assert_eq!(
        rows.iter()
            .filter(|row| row.is_default)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [first.id]
    );

    set_default_provider_config(&db, ProviderConfigKind::Llm, second.id)
        .await
        .unwrap();
    let rows = list_llm_config_records(&db).await.unwrap();
    assert_eq!(
        rows.iter()
            .filter(|row| row.is_default)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [second.id]
    );

    delete_llm_config_record(&db, first.id).await.unwrap();
    delete_llm_config_record(&db, second.id).await.unwrap();
}
