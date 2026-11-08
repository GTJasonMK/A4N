use super::*;
use a4n_api_contract::web_search::WEB_SEARCH_PROTOCOLS;
use a4n_test_support::{
    model_contract::assert_case,
    provider_http::{LocalProvider, ProviderResponse},
};
use serde_json::Value;

fn result(protocol: &str, scenario: &str) -> Value {
    let source = json!({"url":if scenario=="invalid-source" {"file:///tmp/private"} else {"https://example.com/story"},"title":"Story","name":"Story","link":if scenario=="invalid-source" {"file:///tmp/private"} else {"https://example.com/story"}});
    let sources = if scenario == "missing-results" {
        vec![]
    } else {
        vec![source.clone()]
    };
    match protocol {
        "anthropic" => {
            json!({"stop_reason":if scenario=="truncated" {"max_tokens"} else {"end_turn"},"content":[{"type":"web_search_tool_result","content":sources.into_iter().map(|mut source|{source["type"]=json!("web_search_result");source}).collect::<Vec<_>>()}]})
        }
        "openai_responses" => {
            json!({"status":if scenario=="truncated" {"incomplete"} else {"completed"},"output":[{"type":"web_search_call","action":{"sources":sources}}]})
        }
        "tavily" => json!({"results":sources}),
        "brave" => json!({"web":{"results":sources}}),
        "serper" => json!({"organic":sources}),
        "bing" => json!({"webPages":{"value":sources}}),
        _ => panic!("missing production search protocol fixture"),
    }
}

fn config(protocol: &str, url: &str) -> RuntimeWebSearchConfig {
    RuntimeWebSearchConfig {
        protocol: protocol.into(),
        provider: "matrix".into(),
        base_url: url.into(),
        api_key_encrypted: Some("search-key".into()),
        model: "matrix-search".into(),
        max_results: 5,
        max_uses: 2,
        max_tokens: 512,
        timeout_secs: 1,
    }
}

#[tokio::test]
async fn model_contract_matrix_search_real_requests_and_failures() {
    for &protocol in WEB_SEARCH_PROTOCOLS {
        for scenario in [
            "valid",
            "error-envelope",
            "auth",
            "rate-limit",
            "server",
            "malformed-json",
            "duplicate-keys",
            "missing-results",
            "invalid-source",
            "oversize",
            "timeout",
            "missing-finish",
            "truncated",
        ] {
            if matches!(scenario, "missing-finish" | "truncated")
                && !matches!(protocol, "anthropic" | "openai_responses")
            {
                continue;
            }
            let server = LocalProvider::start(move |_| {
                let status = match scenario {
                    "auth" => 401,
                    "rate-limit" => 429,
                    "server" => 503,
                    _ => 200,
                };
                if status != 200 {
                    return ProviderResponse::json(status, json!({"error":"injected fault"}));
                }
                let mut value = result(protocol, scenario);
                if scenario == "error-envelope" {
                    value["error"] = json!({"message":"provider failed despite attached sources"});
                }
                if scenario == "missing-finish" {
                    value
                        .as_object_mut()
                        .unwrap()
                        .remove(if protocol == "anthropic" {
                            "stop_reason"
                        } else {
                            "status"
                        });
                }
                let body = match scenario {
                    "malformed-json" => b"{invalid".to_vec(),
                    "duplicate-keys" => value
                        .to_string()
                        .replacen('{', "{\"_duplicate\":1,\"_duplicate\":2,", 1)
                        .into_bytes(),
                    "oversize" => vec![b' '; MAX_RESPONSE_BYTES + 1],
                    _ => serde_json::to_vec(&value).unwrap(),
                };
                let mut response = ProviderResponse::bytes("application/json", body);
                if scenario == "timeout" {
                    response.delay_ms = 3000;
                }
                response
            })
            .await;
            let result = search_with_config(
                WebSearchInput {
                    query: " matrix query ".into(),
                    max_results: Some(2),
                },
                config(protocol, &server.base_url),
            )
            .await;
            let requests = server.requests();
            assert_eq!(requests.len(), 1, "search requests must not repeat");
            if scenario == "valid" {
                let output = result
                    .as_ref()
                    .unwrap_or_else(|error| panic!("{protocol}: {error}"));
                assert_eq!(output.query, "matrix query");
                assert_eq!(output.sources.len(), 1);
                let body: Value = if requests[0].body.is_empty() {
                    json!({})
                } else {
                    serde_json::from_slice(&requests[0].body).unwrap()
                };
                if matches!(protocol, "anthropic" | "openai_responses") {
                    assert_eq!(body["model"], "matrix-search");
                    assert_eq!(
                        body[if protocol == "anthropic" {
                            "max_tokens"
                        } else {
                            "max_output_tokens"
                        }],
                        512
                    );
                    assert!(body.to_string().contains("matrix query"));
                } else {
                    assert!(
                        String::from_utf8_lossy(&requests[0].body).contains("matrix query")
                            || requests[0].uri.contains("matrix")
                    );
                }
                let key = match protocol {
                    "anthropic" => Some("x-api-key"),
                    "openai_responses" => Some("authorization"),
                    "brave" => Some("x-subscription-token"),
                    "serper" => Some("x-api-key"),
                    "bing" => Some("ocp-apim-subscription-key"),
                    _ => None,
                };
                if let Some(key) = key {
                    assert!(requests[0].headers[key].contains("search-key"));
                } else {
                    assert_eq!(body["api_key"], "search-key");
                }
            }
            assert_case(
                "web_search",
                protocol,
                "query",
                scenario,
                json!(scenario == "valid"),
                json!(result.is_ok()),
                json!({"requests":requests.len(),"uri":requests[0].uri,"error":result.err().map(|error|error.to_string())}),
            );
        }
        let server =
            LocalProvider::start(move |_| ProviderResponse::json(200, result(protocol, "valid")))
                .await;
        let result = search_with_config(
            WebSearchInput {
                query: " ".into(),
                max_results: Some(2),
            },
            config(protocol, &server.base_url),
        )
        .await;
        assert_case(
            "web_search",
            protocol,
            "query",
            "invalid-input",
            json!(true),
            json!(result.is_err() && server.requests().is_empty()),
            json!({"requests":server.requests().len()}),
        );
    }
}
