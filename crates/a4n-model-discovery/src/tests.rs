use reqwest::StatusCode;
use serde_json::json;

use crate::endpoints::{
    ModelDiscoveryAuth, ModelDiscoveryParser, model_discovery_endpoints, openai_model_list_urls,
};
use crate::http::model_discovery_status_error;
use crate::parsing::parse_discovered_models;

#[test]
fn strips_known_openai_suffixes() {
    assert_eq!(
        openai_model_list_urls("https://api.example.com/v1/chat/completions"),
        vec!["https://api.example.com/v1/models"]
    );
}

#[test]
fn routes_anthropic_to_native_model_endpoint_and_auth() {
    let endpoints = model_discovery_endpoints("anthropic", "https://api.anthropic.com");

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].url, "https://api.anthropic.com/v1/models");
    assert!(matches!(endpoints[0].auth, ModelDiscoveryAuth::Anthropic));
}

#[test]
fn parses_openai_model_shapes() {
    let models = parse_discovered_models(
        &json!({
            "data": [
                {"id": "gpt-a", "owned_by": "owner"},
                {"name": "gpt-b", "display_name": "GPT B"},
                "gpt-c"
            ]
        }),
        ModelDiscoveryParser::OpenAi,
    )
    .expect("openai models");

    assert_eq!(models.len(), 3);
    assert_eq!(models[0].id, "gpt-a");
    assert_eq!(models[1].name.as_deref(), Some("gpt-b"));
    assert_eq!(models[2].id, "gpt-c");
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
    assert_eq!(models[0].owned_by.as_deref(), Some("google"));
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
