#[cfg(test)]
mod contract_matrix;
mod response;

use a4n_api_contract::runtime_config::RuntimeWebSearchConfig;
use a4n_api_contract::web_search::{WebSearchInput, WebSearchResult};
use a4n_error::AppError;
use futures::StreamExt;
use serde_json::json;
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

pub async fn web_search(input: WebSearchInput) -> Result<WebSearchResult, AppError> {
    search_with_config(input, current_web_search_config()?).await
}

async fn search_with_config(
    input: WebSearchInput,
    config: RuntimeWebSearchConfig,
) -> Result<WebSearchResult, AppError> {
    let query = input
        .normalized_query()
        .map_err(AppError::InvalidArgument)?;
    a4n_api_contract::web_search::validate_web_search_config(&config)
        .map_err(AppError::ConfigError)?;
    let api_key = config
        .api_key_encrypted
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::ConfigError("联网搜索尚未配置 API Key".into()))?;
    let max_results = input
        .max_results
        .unwrap_or(config.max_results)
        .min(config.max_results);
    let protocol = SearchProtocol::parse(&config.protocol)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(config.timeout_secs as u64))
        .build()
        .map_err(|error| {
            AppError::Internal(anyhow::anyhow!("联网搜索 HTTP 客户端创建失败: {error}"))
        })?;
    let response = protocol
        .request(&client, &config, api_key, &query, max_results)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                AppError::Timeout(format!("联网搜索请求超过 {} 秒", config.timeout_secs))
            } else {
                AppError::Internal(anyhow::anyhow!("联网搜索请求失败: {error}"))
            }
        })?;
    let status = response.status();
    let body = read_response_body(response).await?;
    if !status.is_success() {
        return Err(AppError::LlmError(format!(
            "联网搜索服务返回 HTTP {}: {}",
            status,
            response::response_error_detail(&body, api_key)
        )));
    }
    protocol.parse_response(&query, max_results, &body)
}

fn current_web_search_config() -> Result<RuntimeWebSearchConfig, AppError> {
    crate::runtime_config::current_snapshot()
        .and_then(|snapshot| snapshot.web_search)
        .ok_or_else(|| AppError::ConfigError("个人联网搜索尚未配置".into()))
}

#[derive(Debug, Clone, Copy)]
enum SearchProtocol {
    Anthropic,
    OpenAiResponses,
    Tavily,
    Brave,
    Serper,
    Bing,
}

impl SearchProtocol {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value.trim() {
            "anthropic" => Ok(Self::Anthropic),
            "openai_responses" => Ok(Self::OpenAiResponses),
            "tavily" => Ok(Self::Tavily),
            "brave" => Ok(Self::Brave),
            "serper" => Ok(Self::Serper),
            "bing" => Ok(Self::Bing),
            other => Err(AppError::ConfigError(format!(
                "联网搜索 protocol 不支持: {other}"
            ))),
        }
    }

    fn request<'a>(
        self,
        client: &'a reqwest::Client,
        config: &'a RuntimeWebSearchConfig,
        api_key: &'a str,
        query: &'a str,
        max_results: u32,
    ) -> reqwest::RequestBuilder {
        match self {
            Self::Anthropic => client
                .post(endpoint_for(&config.base_url, "/messages"))
                .header("x-api-key", api_key)
                .header("authorization", format!("Bearer {api_key}"))
                .header("anthropic-version", "2023-06-01")
                .json(&json!({
                    "model": config.model,
                    "max_tokens": config.max_tokens,
                    "messages": [{
                        "role": "user",
                        "content": [{
                            "type": "text",
                            "text": format!("Perform a web search for the query: {query}"),
                        }],
                    }],
                    "tools": [{
                        "type": "web_search_20250305",
                        "name": "web_search",
                        "max_uses": config.max_uses,
                    }],
                })),
            Self::OpenAiResponses => client
                .post(endpoint_for(&config.base_url, "/responses"))
                .bearer_auth(api_key)
                .json(&json!({
                    "model": config.model,
                    "input": format!("Search the web for the query: {query}"),
                    "max_output_tokens": config.max_tokens,
                    "tools": [{"type": "web_search_preview"}],
                })),
            Self::Tavily => client
                .post(endpoint_for(&config.base_url, "/search"))
                .json(&json!({
                    "api_key": api_key,
                    "query": query,
                    "max_results": max_results,
                    "search_depth": "basic",
                    "include_answer": false,
                })),
            Self::Brave => client
                .get(endpoint_for(&config.base_url, "/res/v1/web/search"))
                .header("X-Subscription-Token", api_key)
                .query(&[("q", query), ("count", &max_results.to_string())]),
            Self::Serper => client
                .post(endpoint_for(&config.base_url, "/search"))
                .header("X-API-KEY", api_key)
                .json(&json!({"q": query, "num": max_results})),
            Self::Bing => client
                .get(endpoint_for(&config.base_url, "/v7.0/search"))
                .header("Ocp-Apim-Subscription-Key", api_key)
                .query(&[("q", query), ("count", &max_results.to_string())]),
        }
    }

    fn parse_response(
        self,
        query: &str,
        max_results: u32,
        body: &[u8],
    ) -> Result<WebSearchResult, AppError> {
        match self {
            Self::Anthropic => response::parse_search_response(query, max_results, body),
            Self::OpenAiResponses => response::parse_openai_responses(query, max_results, body),
            Self::Tavily => response::parse_result_array(query, max_results, body, "/results"),
            Self::Brave => response::parse_result_array(query, max_results, body, "/web/results"),
            Self::Bing => response::parse_result_array(query, max_results, body, "/webPages/value"),
            Self::Serper => response::parse_result_array(query, max_results, body, "/organic"),
        }
    }
}

fn endpoint_for(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with(suffix) {
        base.to_string()
    } else {
        format!("{base}{suffix}")
    }
}

async fn read_response_body(response: reqwest::Response) -> Result<Vec<u8>, AppError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(AppError::LlmError(format!(
            "联网搜索响应超过 {} MiB 限制",
            MAX_RESPONSE_BYTES / 1024 / 1024
        )));
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            AppError::Internal(anyhow::anyhow!("读取联网搜索响应失败: {error}"))
        })?;
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(AppError::LlmError(format!(
                "联网搜索响应超过 {} MiB 限制",
                MAX_RESPONSE_BYTES / 1024 / 1024
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::{SearchProtocol, response};

    #[test]
    fn parses_only_structured_search_results_and_joins_citations_after_first_pass() {
        let body = serde_json::json!({
            "stop_reason":"end_turn",
            "content": [
                {
                    "type": "web_search_tool_result",
                    "content": [
                        {
                            "type": "web_search_result",
                            "url": "https://example.com/story",
                            "title": "Story result"
                        },
                        {
                            "type": "web_search_result",
                            "url": "https://example.com/second",
                            "title": "Second"
                        }
                    ]
                },
                {
                    "type": "text",
                    "text": "The answer.",
                    "citations": [{
                        "type": "web_search_result_location",
                        "url": "https://example.com/story",
                        "title": "Story citation",
                        "cited_text": "A cited fact"
                    }]
                }
            ]
        });
        let body = serde_json::to_vec(&body).unwrap();
        let result = response::parse_search_response("query", 5, &body).unwrap();
        assert_eq!(result.sources.len(), 2);
        assert_eq!(result.sources[0].url, "https://example.com/story");
        assert_eq!(result.sources[0].title, "Story result");
        assert_eq!(result.sources[0].snippet.as_deref(), Some("A cited fact"));
        assert!(!result.sources[0].url.contains("text.example"));
    }

    #[test]
    fn rejects_unstructured_model_text_without_search_tool_result() {
        let body = serde_json::json!({
            "stop_reason":"end_turn",
            "content": [{
                "type": "text",
                "text": "https://example.com/not-a-source"
            }]
        });
        let body = serde_json::to_vec(&body).unwrap();
        assert!(response::parse_search_response("query", 5, &body).is_err());
    }

    #[test]
    fn caps_results_and_reports_truncation() {
        let body = serde_json::json!({
            "stop_reason":"end_turn",
            "content": [{
                "type": "web_search_tool_result",
                "content": [
                    {"type": "web_search_result", "url": "https://example.com/1"},
                    {"type": "web_search_result", "url": "https://example.com/2"}
                ]
            }]
        });
        let body = serde_json::to_vec(&body).unwrap();
        let result = response::parse_search_response("query", 1, &body).unwrap();
        assert_eq!(result.sources.len(), 1);
        assert!(result.truncated);
    }

    #[test]
    fn parses_provider_specific_result_paths_and_url_aliases() {
        let brave = serde_json::json!({
            "web": {"results": [{"url": "https://example.com/brave", "title": "Brave", "description": "desc"}]}
        });
        let bing = serde_json::json!({
            "webPages": {"value": [{"url": "https://example.com/bing", "name": "Bing", "snippet": "desc"}]}
        });
        let serper = serde_json::json!({
            "organic": [{"link": "https://example.com/serper", "title": "Serper", "snippet": "desc"}]
        });
        let brave = serde_json::to_vec(&brave).unwrap();
        let bing = serde_json::to_vec(&bing).unwrap();
        let serper = serde_json::to_vec(&serper).unwrap();
        assert_eq!(
            SearchProtocol::Brave
                .parse_response("q", 5, &brave)
                .unwrap()
                .sources[0]
                .url,
            "https://example.com/brave"
        );
        assert_eq!(
            SearchProtocol::Bing
                .parse_response("q", 5, &bing)
                .unwrap()
                .sources[0]
                .title,
            "Bing"
        );
        assert_eq!(
            response::parse_result_array("q", 5, &serper, "/organic")
                .unwrap()
                .sources[0]
                .url,
            "https://example.com/serper"
        );
    }

    #[test]
    fn parses_openai_responses_url_citations() {
        let body = serde_json::json!({
            "status":"completed",
            "output": [{
                "type": "message",
                "content": [{
                    "type": "output_text",
                    "annotations": [{
                        "type": "url_citation",
                        "url": "https://example.com/openai",
                        "title": "OpenAI"
                    }]
                }]
            }]
        });
        let result =
            response::parse_openai_responses("q", 5, &serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(result.sources[0].title, "OpenAI");
    }
}
