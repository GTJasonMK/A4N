use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::serde_update::deserialize_nullable_update_field;

pub const DEFAULT_WEB_SEARCH_PROVIDER: &str = "deepseek-official";
pub const DEFAULT_WEB_SEARCH_PROTOCOL: &str = "anthropic";
pub const DEFAULT_WEB_SEARCH_BASE_URL: &str = "https://api.deepseek.com/anthropic/v1";
pub const DEFAULT_WEB_SEARCH_MODEL: &str = "deepseek-v4-flash";
pub const DEFAULT_WEB_SEARCH_MAX_RESULTS: u32 = 5;
pub const DEFAULT_WEB_SEARCH_MAX_USES: u32 = 5;
pub const DEFAULT_WEB_SEARCH_MAX_TOKENS: u32 = 4096;
pub const DEFAULT_WEB_SEARCH_TIMEOUT_SECS: u32 = 60;
pub const WEB_SEARCH_MAX_RESULTS_LIMIT: u32 = 20;
pub const WEB_SEARCH_MAX_USES_LIMIT: u32 = 20;
pub const WEB_SEARCH_MAX_TOKENS_LIMIT: u32 = 32_768;
pub const WEB_SEARCH_MAX_TIMEOUT_SECS: u32 = 300;
pub const WEB_SEARCH_MAX_QUERY_CHARS: usize = 2_000;

pub const WEB_SEARCH_PROTOCOLS: &[&str] = &[
    "anthropic",
    "openai_responses",
    "tavily",
    "brave",
    "serper",
    "bing",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WebSearchConfigSummary {
    pub protocol: String,
    pub provider: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub model: String,
    pub max_results: u32,
    pub max_uses: u32,
    pub max_tokens: u32,
    pub timeout_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateWebSearchConfigInput {
    #[serde(default = "default_web_search_protocol")]
    pub protocol: String,
    pub provider: String,
    pub base_url: String,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_results: u32,
    pub max_uses: u32,
    pub max_tokens: u32,
    pub timeout_secs: u32,
}

/// Runtime-only configuration. It intentionally has no serde implementation
/// so the saved API key cannot become a normal API response by accident.
#[derive(Debug, Clone, PartialEq)]
pub struct WebSearchRuntimeConfig {
    pub protocol: String,
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_results: u32,
    pub max_uses: u32,
    pub max_tokens: u32,
    pub timeout_secs: u32,
}

impl Default for WebSearchRuntimeConfig {
    fn default() -> Self {
        Self {
            protocol: DEFAULT_WEB_SEARCH_PROTOCOL.to_string(),
            provider: DEFAULT_WEB_SEARCH_PROVIDER.to_string(),
            base_url: DEFAULT_WEB_SEARCH_BASE_URL.to_string(),
            api_key_encrypted: None,
            model: DEFAULT_WEB_SEARCH_MODEL.to_string(),
            max_results: DEFAULT_WEB_SEARCH_MAX_RESULTS,
            max_uses: DEFAULT_WEB_SEARCH_MAX_USES,
            max_tokens: DEFAULT_WEB_SEARCH_MAX_TOKENS,
            timeout_secs: DEFAULT_WEB_SEARCH_TIMEOUT_SECS,
        }
    }
}

impl WebSearchRuntimeConfig {
    pub fn summary(&self) -> WebSearchConfigSummary {
        WebSearchConfigSummary {
            protocol: self.protocol.clone(),
            provider: self.provider.clone(),
            base_url: self.base_url.clone(),
            has_api_key: has_api_key(self.api_key_encrypted.as_deref()),
            model: self.model.clone(),
            max_results: self.max_results,
            max_uses: self.max_uses,
            max_tokens: self.max_tokens,
            timeout_secs: self.timeout_secs,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebSearchInput {
    pub query: String,
    #[serde(default)]
    pub max_results: Option<u32>,
}

impl WebSearchInput {
    pub fn normalized_query(&self) -> Result<String, String> {
        let query = self.query.trim();
        if query.is_empty() {
            return Err("联网搜索 query 不能为空".into());
        }
        if query.contains('\0') {
            return Err("联网搜索 query 不能包含 NUL 字符".into());
        }
        if query.chars().count() > WEB_SEARCH_MAX_QUERY_CHARS {
            return Err(format!(
                "联网搜索 query 不能超过 {WEB_SEARCH_MAX_QUERY_CHARS} 个字符"
            ));
        }
        if self
            .max_results
            .is_some_and(|value| !(1..=WEB_SEARCH_MAX_RESULTS_LIMIT).contains(&value))
        {
            return Err(format!(
                "联网搜索 maxResults 必须在 1-{WEB_SEARCH_MAX_RESULTS_LIMIT} 之间"
            ));
        }
        Ok(query.to_string())
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
pub struct WebSearchResult {
    pub query: String,
    pub sources: Vec<WebSearchSource>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
pub struct WebSearchSource {
    pub url: String,
    pub title: String,
    pub snippet: Option<String>,
    pub published_at: Option<String>,
}

pub fn validate_web_search_config(
    protocol: &str,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    max_results: u32,
    max_uses: u32,
    max_tokens: u32,
    timeout_secs: u32,
) -> Result<(), String> {
    if !WEB_SEARCH_PROTOCOLS.contains(&protocol.trim()) {
        return Err(format!(
            "联网搜索 protocol 不支持: {protocol}；可选值: {}",
            WEB_SEARCH_PROTOCOLS.join(", ")
        ));
    }
    required_text("provider", provider, 64)?;
    required_text("model", model, 256)?;
    let base_url = required_text("base_url", base_url, 2_048)?;
    let parsed = url::Url::parse(base_url).map_err(|error| format!("base_url 无效: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("base_url 必须是带 host 的 http:// 或 https:// URL".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("base_url 不能包含用户名或密码".into());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("base_url 不能包含查询参数或 fragment".into());
    }
    if api_key.is_some_and(|value| value.trim().is_empty()) {
        return Err("api_key_encrypted 不能是空字符串".into());
    }
    if !(1..=WEB_SEARCH_MAX_RESULTS_LIMIT).contains(&max_results) {
        return Err(format!(
            "max_results 必须在 1-{WEB_SEARCH_MAX_RESULTS_LIMIT} 之间"
        ));
    }
    if !(1..=WEB_SEARCH_MAX_USES_LIMIT).contains(&max_uses) {
        return Err(format!(
            "max_uses 必须在 1-{WEB_SEARCH_MAX_USES_LIMIT} 之间"
        ));
    }
    if !(1..=WEB_SEARCH_MAX_TOKENS_LIMIT).contains(&max_tokens) {
        return Err(format!(
            "max_tokens 必须在 1-{WEB_SEARCH_MAX_TOKENS_LIMIT} 之间"
        ));
    }
    if !(1..=WEB_SEARCH_MAX_TIMEOUT_SECS).contains(&timeout_secs) {
        return Err(format!(
            "timeout_secs 必须在 1-{WEB_SEARCH_MAX_TIMEOUT_SECS} 之间"
        ));
    }
    Ok(())
}

fn default_web_search_protocol() -> String {
    DEFAULT_WEB_SEARCH_PROTOCOL.to_string()
}

fn required_text<'a>(field: &str, value: &'a str, max_chars: usize) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} 不能为空"));
    }
    if value.contains('\0') {
        return Err(format!("{field} 不能包含 NUL 字符"));
    }
    if value.chars().count() > max_chars {
        return Err(format!("{field} 不能超过 {max_chars} 个字符"));
    }
    Ok(value)
}

fn has_api_key(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_WEB_SEARCH_BASE_URL, DEFAULT_WEB_SEARCH_MAX_RESULTS, DEFAULT_WEB_SEARCH_MAX_TOKENS,
        DEFAULT_WEB_SEARCH_MAX_USES, DEFAULT_WEB_SEARCH_MODEL, DEFAULT_WEB_SEARCH_PROTOCOL,
        DEFAULT_WEB_SEARCH_PROVIDER, DEFAULT_WEB_SEARCH_TIMEOUT_SECS, UpdateWebSearchConfigInput,
        WebSearchInput, WebSearchRuntimeConfig, validate_web_search_config,
    };
    use serde_json::json;

    #[test]
    fn web_search_input_uses_camel_case_and_rejects_unknown_fields() {
        let input: WebSearchInput = serde_json::from_value(json!({
            "query": "  current events  ",
            "maxResults": 3,
        }))
        .unwrap();
        assert_eq!(input.normalized_query().unwrap(), "current events");
        assert!(
            serde_json::from_value::<WebSearchInput>(json!({
                "query": "test",
                "max_results": 3,
            }))
            .is_err()
        );
    }

    #[test]
    fn api_key_update_distinguishes_omitted_and_clear() {
        let base = json!({
            "protocol": DEFAULT_WEB_SEARCH_PROTOCOL,
            "provider": DEFAULT_WEB_SEARCH_PROVIDER,
            "base_url": DEFAULT_WEB_SEARCH_BASE_URL,
            "model": DEFAULT_WEB_SEARCH_MODEL,
            "max_results": DEFAULT_WEB_SEARCH_MAX_RESULTS,
            "max_uses": DEFAULT_WEB_SEARCH_MAX_USES,
            "max_tokens": DEFAULT_WEB_SEARCH_MAX_TOKENS,
            "timeout_secs": DEFAULT_WEB_SEARCH_TIMEOUT_SECS,
        });
        let omitted: UpdateWebSearchConfigInput = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(omitted.api_key_encrypted, None);
        let mut cleared = base;
        cleared["api_key_encrypted"] = serde_json::Value::Null;
        let cleared: UpdateWebSearchConfigInput = serde_json::from_value(cleared).unwrap();
        assert_eq!(cleared.api_key_encrypted, Some(None));
    }

    #[test]
    fn config_validation_rejects_unsafe_endpoint_and_invalid_limits() {
        assert!(
            validate_web_search_config(
                DEFAULT_WEB_SEARCH_PROTOCOL,
                DEFAULT_WEB_SEARCH_PROVIDER,
                "ftp://example.com",
                Some("secret"),
                DEFAULT_WEB_SEARCH_MODEL,
                DEFAULT_WEB_SEARCH_MAX_RESULTS,
                DEFAULT_WEB_SEARCH_MAX_USES,
                DEFAULT_WEB_SEARCH_MAX_TOKENS,
                DEFAULT_WEB_SEARCH_TIMEOUT_SECS,
            )
            .is_err()
        );
        assert!(
            validate_web_search_config(
                DEFAULT_WEB_SEARCH_PROTOCOL,
                DEFAULT_WEB_SEARCH_PROVIDER,
                DEFAULT_WEB_SEARCH_BASE_URL,
                Some("secret"),
                DEFAULT_WEB_SEARCH_MODEL,
                0,
                DEFAULT_WEB_SEARCH_MAX_USES,
                DEFAULT_WEB_SEARCH_MAX_TOKENS,
                DEFAULT_WEB_SEARCH_TIMEOUT_SECS,
            )
            .is_err()
        );
    }

    #[test]
    fn runtime_summary_only_exposes_key_presence() {
        let config = WebSearchRuntimeConfig {
            api_key_encrypted: Some("secret".into()),
            ..Default::default()
        };
        let summary = config.summary();
        assert!(summary.has_api_key);
        assert!(!serde_json::to_string(&summary).unwrap().contains("secret"));
    }
}
