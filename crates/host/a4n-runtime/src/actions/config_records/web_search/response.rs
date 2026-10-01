use a4n_api_contract::web_search::{WebSearchResult, WebSearchSource};
use a4n_error::AppError;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub(super) fn parse_search_response(
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

pub(super) fn parse_result_array(
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

pub(super) fn parse_openai_responses(
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

fn citation_snippets(blocks: &[Value]) -> HashMap<&str, &str> {
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

pub(super) fn response_error_detail(body: &[u8], api_key: &str) -> String {
    let text = String::from_utf8_lossy(body).trim().to_string();
    if text.is_empty() {
        return "服务未返回错误详情".into();
    }
    text.replace(api_key, "[redacted]")
        .chars()
        .take(512)
        .collect()
}
