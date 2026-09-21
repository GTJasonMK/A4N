use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::runtime_config::RuntimeWebSearchConfig;

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

pub fn validate_web_search_config(config: &RuntimeWebSearchConfig) -> Result<(), String> {
    if !WEB_SEARCH_PROTOCOLS.contains(&config.protocol.trim()) {
        return Err(format!(
            "联网搜索 protocol 不支持: {}；可选值: {}",
            config.protocol,
            WEB_SEARCH_PROTOCOLS.join(", ")
        ));
    }
    required_text("provider", &config.provider, 64)?;
    required_text("model", &config.model, 256)?;
    let base_url = required_text("base_url", &config.base_url, 2_048)?;
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
    if config
        .api_key_encrypted
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err("api_key_encrypted 不能是空字符串".into());
    }
    if !(1..=WEB_SEARCH_MAX_RESULTS_LIMIT).contains(&config.max_results) {
        return Err(format!(
            "max_results 必须在 1-{WEB_SEARCH_MAX_RESULTS_LIMIT} 之间"
        ));
    }
    if !(1..=WEB_SEARCH_MAX_USES_LIMIT).contains(&config.max_uses) {
        return Err(format!(
            "max_uses 必须在 1-{WEB_SEARCH_MAX_USES_LIMIT} 之间"
        ));
    }
    if !(1..=WEB_SEARCH_MAX_TOKENS_LIMIT).contains(&config.max_tokens) {
        return Err(format!(
            "max_tokens 必须在 1-{WEB_SEARCH_MAX_TOKENS_LIMIT} 之间"
        ));
    }
    if !(1..=WEB_SEARCH_MAX_TIMEOUT_SECS).contains(&config.timeout_secs) {
        return Err(format!(
            "timeout_secs 必须在 1-{WEB_SEARCH_MAX_TIMEOUT_SECS} 之间"
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{WebSearchInput, validate_web_search_config};
    use crate::runtime_config::RuntimeWebSearchConfig;
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
    fn config_validation_rejects_unsafe_endpoint_and_invalid_limits() {
        let mut config = valid_runtime_config();
        config.base_url = "ftp://example.com".into();
        assert!(validate_web_search_config(&config).is_err());
        config = valid_runtime_config();
        config.max_results = 0;
        assert!(validate_web_search_config(&config).is_err());
    }

    fn valid_runtime_config() -> RuntimeWebSearchConfig {
        RuntimeWebSearchConfig {
            protocol: "anthropic".into(),
            provider: "deepseek-official".into(),
            base_url: "https://api.deepseek.com/anthropic/v1".into(),
            api_key_encrypted: Some("secret".into()),
            model: "deepseek-v4-flash".into(),
            max_results: 5,
            max_uses: 5,
            max_tokens: 4096,
            timeout_secs: 60,
        }
    }
}
