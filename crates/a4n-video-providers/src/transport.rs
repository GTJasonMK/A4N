use serde_json::Value;

use a4n_core::error::AppError;

use crate::idempotency::with_provider_idempotency_key;
use crate::response::{parse_json_response, parse_json_response_allow_not_found};

/// Provider-specific key handling is intentionally explicit: some legacy
/// endpoints receive the raw value, while others require trimming or a
/// non-Bearer header.
#[derive(Clone, Copy)]
pub(crate) enum ApiKeyAuth {
    Bearer,
    TrimmedBearer,
    TrimmedHeader(&'static str),
}

pub(crate) fn apply_api_key(
    request: reqwest::RequestBuilder,
    api_key: &str,
    auth: ApiKeyAuth,
) -> reqwest::RequestBuilder {
    match auth {
        ApiKeyAuth::Bearer if !api_key.is_empty() => request.bearer_auth(api_key),
        ApiKeyAuth::TrimmedBearer if !api_key.trim().is_empty() => {
            request.bearer_auth(api_key.trim())
        }
        ApiKeyAuth::TrimmedHeader(name) if !api_key.trim().is_empty() => {
            request.header(name, api_key.trim())
        }
        _ => request,
    }
}

pub(crate) async fn post_json(
    client: &reqwest::Client,
    api_key: &str,
    auth: ApiKeyAuth,
    url: &str,
    body: &Value,
    provider_idempotency_key_sha256: Option<&str>,
    label: &str,
) -> Result<Value, AppError> {
    let request = apply_api_key(client.post(url).json(body), api_key, auth);
    let request = with_provider_idempotency_key(request, provider_idempotency_key_sha256)?;
    let response = send(request, label).await?;
    parse_json_response(response, label).await
}

pub(crate) async fn get_json(
    client: &reqwest::Client,
    api_key: &str,
    auth: ApiKeyAuth,
    url: &str,
    label: &str,
) -> Result<Value, AppError> {
    let response = send(apply_api_key(client.get(url), api_key, auth), label).await?;
    parse_json_response(response, label).await
}

pub(crate) async fn get_status_json(
    client: &reqwest::Client,
    api_key: &str,
    auth: ApiKeyAuth,
    url: &str,
    label: &str,
) -> Result<Option<Value>, AppError> {
    let response = send(apply_api_key(client.get(url), api_key, auth), label).await?;
    parse_json_response_allow_not_found(response, label).await
}

async fn send(
    request: reqwest::RequestBuilder,
    label: &str,
) -> Result<reqwest::Response, AppError> {
    request
        .send()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("{label}请求失败：{error}")))
}

#[cfg(test)]
mod tests {
    use reqwest::header::AUTHORIZATION;

    use super::*;

    #[test]
    fn preserves_each_api_key_auth_policy() {
        let raw = apply_api_key(
            reqwest::Client::new().get("https://example.test"),
            "raw-key",
            ApiKeyAuth::Bearer,
        )
        .build()
        .unwrap();
        assert_eq!(raw.headers()[AUTHORIZATION], "Bearer raw-key");

        let trimmed = apply_api_key(
            reqwest::Client::new().get("https://example.test"),
            "  trimmed-key  ",
            ApiKeyAuth::TrimmedBearer,
        )
        .build()
        .unwrap();
        assert_eq!(trimmed.headers()[AUTHORIZATION], "Bearer trimmed-key");

        let google = apply_api_key(
            reqwest::Client::new().get("https://example.test"),
            "  google-key  ",
            ApiKeyAuth::TrimmedHeader("x-goog-api-key"),
        )
        .build()
        .unwrap();
        assert_eq!(google.headers()["x-goog-api-key"], "google-key");
    }
}
