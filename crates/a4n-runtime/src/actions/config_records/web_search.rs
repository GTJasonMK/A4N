use crate::{AppState, actions::app_error_message};
use a4n_api_contract::web_search::{
    WebSearchConfigSummary, WebSearchInput, WebSearchResult, WebSearchRuntimeConfig,
    WebSearchSource,
};
use a4n_error::AppError;
use futures::StreamExt;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

pub async fn get_web_search_config(
    state: &AppState,
    caller_user_id: i64,
) -> Result<WebSearchConfigSummary, String> {
    state
        .config_application()
        .web_search_config_summary(caller_user_id)
        .await
        .map_err(app_error_message)
}

pub async fn update_web_search_config(
    state: &AppState,
    caller_user_id: i64,
    input: a4n_api_contract::web_search::UpdateWebSearchConfigInput,
) -> Result<WebSearchConfigSummary, AppError> {
    state
        .config_application()
        .update_web_search_config(caller_user_id, input)
        .await
}

pub async fn web_search(
    state: &AppState,
    input: WebSearchInput,
) -> Result<WebSearchResult, AppError> {
    let query = input
        .normalized_query()
        .map_err(AppError::InvalidArgument)?;
    let config = state
        .config_application()
        .web_search_runtime_config()
        .await?;
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
            response_error_detail(&body, api_key)
        )));
    }
    protocol.parse_response(&query, max_results, &body)
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
        config: &'a WebSearchRuntimeConfig,
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
            Self::Anthropic => parse_search_response(query, max_results, body),
            Self::OpenAiResponses => parse_openai_responses(query, max_results, body),
            Self::Tavily => parse_result_array(query, max_results, body, "/results"),
            Self::Brave => parse_result_array(query, max_results, body, "/web/results"),
            Self::Bing => parse_result_array(query, max_results, body, "/webPages/value"),
            Self::Serper => parse_result_array(query, max_results, body, "/organic"),
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

fn parse_search_response(
    query: &str,
    max_results: u32,
    body: &[u8],
) -> Result<WebSearchResult, AppError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| AppError::LlmError(format!("联网搜索响应不是合法 JSON: {error}")))?;
    let blocks = value
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::LlmError("联网搜索响应缺少 content 结构".into()))?;
    let mut saw_tool_result = false;
    let mut sources = Vec::new();
    let mut seen_urls = HashSet::new();
    let citation_snippets = citation_snippets(blocks);

    for block in blocks {
        if block.get("type").and_then(Value::as_str) != Some("web_search_tool_result") {
            continue;
        }
        saw_tool_result = true;
        for item in values_in(block.get("content")) {
            if item.get("type").and_then(Value::as_str) == Some("web_search_result") {
                let snippet = item
                    .get("url")
                    .and_then(Value::as_str)
                    .and_then(|url| citation_snippets.get(url).copied());
                add_source(item, snippet, &mut sources, &mut seen_urls)?;
            }
        }
    }

    if !saw_tool_result {
        return Err(AppError::LlmError(
            "联网搜索响应缺少 web_search_tool_result".into(),
        ));
    }
    if sources.is_empty() {
        return Err(AppError::LlmError(
            "联网搜索响应没有可用的结构化来源".into(),
        ));
    }
    let truncated = sources.len() > max_results as usize;
    sources.truncate(max_results as usize);
    Ok(WebSearchResult {
        query: query.to_string(),
        sources,
        truncated,
    })
}

fn parse_result_array(
    query: &str,
    max_results: u32,
    body: &[u8],
    pointer: &str,
) -> Result<WebSearchResult, AppError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| AppError::LlmError(format!("联网搜索响应不是合法 JSON: {error}")))?;
    let results = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::LlmError(format!("联网搜索响应缺少 {pointer} 结构")))?;
    let mut sources = Vec::new();
    let mut seen_urls = HashSet::new();
    for result in results {
        add_source(result, None, &mut sources, &mut seen_urls)?;
    }
    if sources.is_empty() {
        return Err(AppError::LlmError(
            "联网搜索响应没有可用的结构化来源".into(),
        ));
    }
    let truncated = sources.len() > max_results as usize;
    sources.truncate(max_results as usize);
    Ok(WebSearchResult {
        query: query.to_string(),
        sources,
        truncated,
    })
}

fn parse_openai_responses(
    query: &str,
    max_results: u32,
    body: &[u8],
) -> Result<WebSearchResult, AppError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| AppError::LlmError(format!("联网搜索响应不是合法 JSON: {error}")))?;
    let output = value
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::LlmError("OpenAI Responses 响应缺少 output 结构".into()))?;
    let mut sources = Vec::new();
    let mut seen_urls = HashSet::new();
    for item in output {
        if item.get("type").and_then(Value::as_str) == Some("message") {
            for content in values_in(item.get("content")) {
                for citation in values_in(content.get("annotations")) {
                    if citation.get("type").and_then(Value::as_str) != Some("url_citation") {
                        continue;
                    }
                    let source = json!({
                        "url": citation.get("url"),
                        "title": citation.get("title"),
                    });
                    add_source(&source, None, &mut sources, &mut seen_urls)?;
                }
            }
        }
        if item.get("type").and_then(Value::as_str) == Some("web_search_call") {
            for source in values_in(item.pointer("/action/sources")) {
                add_source(source, None, &mut sources, &mut seen_urls)?;
            }
        }
    }
    if sources.is_empty() {
        return Err(AppError::LlmError(
            "OpenAI Responses 响应没有 url_citation 结构化来源".into(),
        ));
    }
    let truncated = sources.len() > max_results as usize;
    sources.truncate(max_results as usize);
    Ok(WebSearchResult {
        query: query.to_string(),
        sources,
        truncated,
    })
}

fn citation_snippets<'a>(blocks: &'a [Value]) -> HashMap<&'a str, &'a str> {
    let mut snippets = HashMap::new();
    for block in blocks {
        if block.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        for citation in values_in(block.get("citations")) {
            let Some(url) = citation.get("url").and_then(Value::as_str) else {
                continue;
            };
            let Some(snippet) = citation.get("cited_text").and_then(Value::as_str) else {
                continue;
            };
            let url = url.trim();
            let snippet = snippet.trim();
            if !url.is_empty() && !snippet.is_empty() {
                snippets.entry(url).or_insert(snippet);
            }
        }
    }
    snippets
}

fn values_in(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(values)) => values.iter().collect(),
        Some(value @ Value::Object(_)) => vec![value],
        _ => Vec::new(),
    }
}

fn add_source(
    value: &Value,
    citation_snippet: Option<&str>,
    sources: &mut Vec<WebSearchSource>,
    seen_urls: &mut HashSet<String>,
) -> Result<(), AppError> {
    let Some(raw_url) = value
        .get("url")
        .or_else(|| value.get("link"))
        .and_then(Value::as_str)
    else {
        return Ok(());
    };
    let parsed = reqwest::Url::parse(raw_url)
        .map_err(|error| AppError::LlmError(format!("联网搜索返回了无效来源 URL: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AppError::LlmError(
            "联网搜索返回了非 HTTP(S) 来源 URL".into(),
        ));
    }
    let url = parsed.to_string();
    if !seen_urls.insert(url.clone()) {
        return Ok(());
    }
    let title = value
        .get("title")
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&url)
        .to_string();
    let snippet = first_string(
        value,
        &["snippet", "description", "content", "cited_text", "text"],
    )
    .or_else(|| citation_snippet.map(str::to_string));
    let published_at = first_string(
        value,
        &[
            "published_at",
            "publishedAt",
            "published_date",
            "datePublished",
            "date",
            "page_age",
        ],
    );
    sources.push(WebSearchSource {
        url,
        title,
        snippet,
        published_at,
    });
    Ok(())
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn response_error_detail(body: &[u8], api_key: &str) -> String {
    let text = String::from_utf8_lossy(body).trim().to_string();
    if text.is_empty() {
        return "服务未返回错误详情".into();
    }
    text.replace(api_key, "[redacted]")
        .chars()
        .take(512)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        SearchProtocol, parse_openai_responses, parse_result_array, parse_search_response,
    };

    #[test]
    fn parses_only_structured_search_results_and_joins_citations_after_first_pass() {
        let body = serde_json::json!({
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
        let result = parse_search_response("query", 5, &body).unwrap();
        assert_eq!(result.sources.len(), 2);
        assert_eq!(result.sources[0].url, "https://example.com/story");
        assert_eq!(result.sources[0].title, "Story result");
        assert_eq!(result.sources[0].snippet.as_deref(), Some("A cited fact"));
        assert!(!result.sources[0].url.contains("text.example"));
    }

    #[test]
    fn rejects_unstructured_model_text_without_search_tool_result() {
        let body = serde_json::json!({
            "content": [{
                "type": "text",
                "text": "https://example.com/not-a-source"
            }]
        });
        let body = serde_json::to_vec(&body).unwrap();
        assert!(parse_search_response("query", 5, &body).is_err());
    }

    #[test]
    fn caps_results_and_reports_truncation() {
        let body = serde_json::json!({
            "content": [{
                "type": "web_search_tool_result",
                "content": [
                    {"type": "web_search_result", "url": "https://example.com/1"},
                    {"type": "web_search_result", "url": "https://example.com/2"}
                ]
            }]
        });
        let body = serde_json::to_vec(&body).unwrap();
        let result = parse_search_response("query", 1, &body).unwrap();
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
            parse_result_array("q", 5, &serper, "/organic")
                .unwrap()
                .sources[0]
                .url,
            "https://example.com/serper"
        );
    }

    #[test]
    fn parses_openai_responses_url_citations() {
        let body = serde_json::json!({
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
        let result = parse_openai_responses("q", 5, &serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(result.sources[0].title, "OpenAI");
    }
}
