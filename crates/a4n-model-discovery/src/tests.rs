use reqwest::StatusCode;
use serde_json::json;

use crate::endpoints::{
    ModelDiscoveryAuth, ModelDiscoveryParser, model_discovery_endpoint, openai_model_list_url,
};
use crate::http::model_discovery_status_error;
use crate::parsing::parse_discovered_models;

#[test]
fn strips_known_openai_suffixes() {
    assert_eq!(
        openai_model_list_url("https://api.example.com/v1/chat/completions"),
        "https://api.example.com/v1/models"
    );
}

#[test]
fn routes_anthropic_to_native_model_endpoint_and_auth() {
    let endpoint = model_discovery_endpoint("anthropic", "https://api.anthropic.com");

    assert_eq!(endpoint.url, "https://api.anthropic.com/v1/models");
    assert!(matches!(endpoint.auth, ModelDiscoveryAuth::Anthropic));
}

#[test]
fn parses_provider_data_list() {
    let models = parse_discovered_models(
        &json!({
            "data": [
                {"id": "gpt-a", "owned_by": "owner"},
                {"id": "gpt-b", "display_name": "GPT B"}
            ]
        }),
        ModelDiscoveryParser::DataList,
    )
    .expect("provider data list");

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "gpt-a");
    assert_eq!(models[1].name.as_deref(), Some("GPT B"));
}

#[test]
fn parses_gemini_model_names_without_prefix() {
    let models = parse_discovered_models(
        &json!({
            "models": [
                {"name": "models/gemini-2.0-flash", "displayName": "Gemini Flash"}
            ]
        }),
        ModelDiscoveryParser::Gemini,
    )
    .expect("gemini models");

    assert_eq!(models[0].id, "gemini-2.0-flash");
    assert_eq!(models[0].name.as_deref(), Some("Gemini Flash"));
}

#[test]
fn status_errors_do_not_echo_url() {
    let message = model_discovery_status_error(
        "OpenAI-compatible 模型列表端点",
        StatusCode::UNAUTHORIZED,
        r#"{"error":{"message":"bad key","code":"unauthorized"}}"#,
    );

    assert!(message.contains("OpenAI-compatible 模型列表端点 返回 401 Unauthorized"));
    assert!(message.contains("bad key; code=unauthorized"));
    assert!(!message.contains("https://"));
}
