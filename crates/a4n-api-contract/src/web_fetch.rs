use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::runtime_config::RuntimeWebFetchConfig;

pub const WEB_FETCH_MAX_URL_CHARS: usize = 2_048;
pub const DEFAULT_WEB_FETCH_ENABLED: bool = true;
pub const DEFAULT_WEB_FETCH_TIMEOUT_SECS: u32 = 30;
pub const DEFAULT_WEB_FETCH_MAX_REDIRECTS: u32 = 5;
pub const DEFAULT_WEB_FETCH_MAX_RESPONSE_BYTES: u64 = 5_000_000;
pub const DEFAULT_WEB_FETCH_MAX_BODY_CHARS: u32 = 100_000;
pub const DEFAULT_WEB_FETCH_MAX_OUTPUT_CHARS: u32 = 200_000;
pub const WEB_FETCH_MAX_TIMEOUT_SECS: u32 = 300;
pub const WEB_FETCH_MAX_REDIRECTS: u32 = 10;
pub const WEB_FETCH_MAX_RESPONSE_BYTES: u64 = 10_000_000;
pub const WEB_FETCH_MAX_BODY_CHARS: u32 = 200_000;
pub const WEB_FETCH_MAX_OUTPUT_CHARS: u32 = 200_000;

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WebFetchInput {
    pub url: String,
}

impl WebFetchInput {
    pub fn normalized_url(&self) -> Result<url::Url, String> {
        let url = self.url.trim();
        if url.is_empty() {
            return Err("网页读取 url 不能为空".into());
        }
        if url.chars().count() > WEB_FETCH_MAX_URL_CHARS {
            return Err(format!(
                "网页读取 url 不能超过 {WEB_FETCH_MAX_URL_CHARS} 个字符"
            ));
        }
        let parsed = url::Url::parse(url).map_err(|error| format!("网页读取 url 无效: {error}"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("网页读取只支持 http:// 或 https:// URL".into());
        }
        if parsed.host_str().is_none() {
            return Err("网页读取 url 必须包含 host".into());
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err("网页读取 url 不能包含用户名或密码".into());
        }
        Ok(parsed)
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct WebFetchResult {
    pub url: String,
    pub status_code: u16,
    pub content: String,
    pub truncated: bool,
}

pub fn validate_web_fetch_config(config: &RuntimeWebFetchConfig) -> Result<(), String> {
    if !(1..=WEB_FETCH_MAX_TIMEOUT_SECS).contains(&config.timeout_secs) {
        return Err(format!(
            "web_fetch timeout_secs 必须在 1-{WEB_FETCH_MAX_TIMEOUT_SECS} 之间"
        ));
    }
    if config.max_redirects > WEB_FETCH_MAX_REDIRECTS {
        return Err(format!(
            "web_fetch max_redirects 不能超过 {WEB_FETCH_MAX_REDIRECTS}"
        ));
    }
    if !(1..=WEB_FETCH_MAX_RESPONSE_BYTES).contains(&config.max_response_bytes) {
        return Err(format!(
            "web_fetch max_response_bytes 必须在 1-{WEB_FETCH_MAX_RESPONSE_BYTES} 之间"
        ));
    }
    if !(1..=WEB_FETCH_MAX_BODY_CHARS).contains(&config.max_body_chars) {
        return Err(format!(
            "web_fetch max_body_chars 必须在 1-{WEB_FETCH_MAX_BODY_CHARS} 之间"
        ));
    }
    if !(1..=WEB_FETCH_MAX_OUTPUT_CHARS).contains(&config.max_output_chars) {
        return Err(format!(
            "web_fetch max_output_chars 必须在 1-{WEB_FETCH_MAX_OUTPUT_CHARS} 之间"
        ));
    }
    Ok(())
}

pub fn default_web_fetch_config() -> RuntimeWebFetchConfig {
    RuntimeWebFetchConfig {
        enabled: DEFAULT_WEB_FETCH_ENABLED,
        timeout_secs: DEFAULT_WEB_FETCH_TIMEOUT_SECS,
        max_redirects: DEFAULT_WEB_FETCH_MAX_REDIRECTS,
        max_response_bytes: DEFAULT_WEB_FETCH_MAX_RESPONSE_BYTES,
        max_body_chars: DEFAULT_WEB_FETCH_MAX_BODY_CHARS,
        max_output_chars: DEFAULT_WEB_FETCH_MAX_OUTPUT_CHARS,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_WEB_FETCH_MAX_BODY_CHARS, DEFAULT_WEB_FETCH_MAX_OUTPUT_CHARS,
        DEFAULT_WEB_FETCH_MAX_REDIRECTS, DEFAULT_WEB_FETCH_MAX_RESPONSE_BYTES,
        DEFAULT_WEB_FETCH_TIMEOUT_SECS, WebFetchInput, validate_web_fetch_config,
    };
    use crate::runtime_config::RuntimeWebFetchConfig;

    #[test]
    fn normalizes_public_http_url_shape_without_accepting_credentials() {
        let input = WebFetchInput {
            url: "  https://example.com/article?q=1  ".into(),
        };
        assert_eq!(
            input.normalized_url().unwrap().as_str(),
            "https://example.com/article?q=1"
        );
        assert!(
            WebFetchInput {
                url: "https://user:pass@example.com".into(),
            }
            .normalized_url()
            .is_err()
        );
    }

    #[test]
    fn rejects_non_http_urls_and_blank_input() {
        for url in ["", "file:///tmp/page.html", "ftp://example.com"] {
            assert!(WebFetchInput { url: url.into() }.normalized_url().is_err());
        }
    }

    #[test]
    fn validates_fetch_limits() {
        let mut config = RuntimeWebFetchConfig {
            enabled: true,
            timeout_secs: DEFAULT_WEB_FETCH_TIMEOUT_SECS,
            max_redirects: DEFAULT_WEB_FETCH_MAX_REDIRECTS,
            max_response_bytes: DEFAULT_WEB_FETCH_MAX_RESPONSE_BYTES,
            max_body_chars: DEFAULT_WEB_FETCH_MAX_BODY_CHARS,
            max_output_chars: DEFAULT_WEB_FETCH_MAX_OUTPUT_CHARS,
        };
        validate_web_fetch_config(&config).unwrap();
        config.max_redirects = 11;
        assert!(validate_web_fetch_config(&config).is_err());
    }
}
