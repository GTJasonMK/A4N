use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde_json::Value;

use crate::contracts::{DiscoveredModel, ModelDiscoveryRequest, ModelDiscoveryResult};
use crate::endpoints::{ModelDiscoveryAuth, ModelDiscoveryEndpoint, model_discovery_endpoint};
use crate::parsing::parse_discovered_models;

const MODEL_DISCOVERY_CONNECT_TIMEOUT_SECS: u64 = 20;
const MODEL_DISCOVERY_READ_TIMEOUT_SECS: u64 = 60;
const MODEL_DISCOVERY_ERROR_PREVIEW_CHARS: usize = 500;

pub async fn discover_models_from_request(
    request: ModelDiscoveryRequest,
) -> Result<ModelDiscoveryResult, String> {
    discover_models(
        &request.provider,
        &request.base_url,
        request.api_key.as_deref(),
    )
    .await
}

pub async fn discover_models(
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<ModelDiscoveryResult, String> {
    let base_url = required_model_discovery_base_url(base_url)?;
    let endpoint = model_discovery_endpoint(provider, &base_url);
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(MODEL_DISCOVERY_CONNECT_TIMEOUT_SECS))
        .read_timeout(Duration::from_secs(MODEL_DISCOVERY_READ_TIMEOUT_SECS))
        .build()
        .map_err(|error| format!("模型探测 HTTP 客户端初始化失败：{error}"))?;

    let models = fetch_discovered_models(&client, &endpoint, api_key).await?;
    if models.is_empty() {
        return Err(format!("{}：响应中没有可识别模型", endpoint.label));
    }
    Ok(ModelDiscoveryResult { models })
}

fn required_model_discovery_base_url(base_url: &str) -> Result<String, String> {
    let base_url = base_url.trim().trim_end_matches('/').to_string();
    if base_url.is_empty() {
        return Err("请先填写 API Base URL".to_string());
    }
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("API Base URL 必须以 http:// 或 https:// 开头".to_string());
    }
    Ok(base_url)
}

async fn fetch_discovered_models(
    client: &Client,
    endpoint: &ModelDiscoveryEndpoint,
    api_key: Option<&str>,
) -> Result<Vec<DiscoveredModel>, String> {
    let mut request = client
        .get(&endpoint.url)
        .header("accept", "application/json");
    match endpoint.auth {
        ModelDiscoveryAuth::Bearer => {
            if let Some(api_key) = api_key {
                request = request.bearer_auth(api_key);
            }
        }
        ModelDiscoveryAuth::Anthropic => {
            if let Some(api_key) = api_key {
                request = request.header("x-api-key", api_key);
            }
            request = request.header("anthropic-version", "2023-06-01");
        }
        ModelDiscoveryAuth::GeminiApiKey => {
            if let Some(api_key) = api_key {
                request = request.header("x-goog-api-key", api_key);
            }
        }
        ModelDiscoveryAuth::None => {}
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("{} 请求失败：{error}", endpoint.label))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("{} 响应读取失败：{error}", endpoint.label))?;
    if !status.is_success() {
        return Err(model_discovery_status_error(endpoint.label, status, &text));
    }
    let json: Value = serde_json::from_str(&text).map_err(|error| {
        format!(
            "{} 响应不是合法 JSON：{error}；响应片段：{}",
            endpoint.label,
            preview_model_discovery_body(&text)
        )
    })?;
    parse_discovered_models(&json, endpoint.parser).map_err(|error| {
        format!(
            "{} 响应结构无法识别：{error}；响应片段：{}",
            endpoint.label,
            preview_model_discovery_body(&text)
        )
    })
}

pub(crate) fn model_discovery_status_error(label: &str, status: StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| model_discovery_upstream_error(&value))
        .unwrap_or_else(|| preview_model_discovery_body(body));
    format!("{label} 返回 {status}：{detail}")
}

fn model_discovery_upstream_error(value: &Value) -> Option<String> {
    let error = value.get("error").unwrap_or(value);
    if let Some(message) = error
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(message.to_string());
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    Some(match code {
        Some(code) => format!("{message}; code={code}"),
        None => message.to_string(),
    })
}

fn preview_model_discovery_body(body: &str) -> String {
    body.chars()
        .take(MODEL_DISCOVERY_ERROR_PREVIEW_CHARS)
        .collect()
}
