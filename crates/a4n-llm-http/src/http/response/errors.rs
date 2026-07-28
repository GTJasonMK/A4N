use super::super::errors::{internal_error, response_preview};
use super::json::{json_response_preview, non_empty_string};
use a4n_core::error::AppError;
use reqwest::StatusCode;
use serde_json::Value;

pub(super) fn upstream_error_message(json: &Value) -> Option<String> {
    if let Some(error) = json.get("error") {
        return match error {
            Value::String(message) => {
                let message = message.trim();
                (!message.is_empty()).then(|| message.to_string())
            }
            Value::Object(error) => {
                let message = error.get("message").and_then(non_empty_string);
                let error_type = error.get("type").and_then(non_empty_string);
                let code = error.get("code").and_then(non_empty_string);

                let mut parts = Vec::new();
                if let Some(message) = message {
                    parts.push(message.to_string());
                }
                if let Some(error_type) = error_type {
                    parts.push(format!("type={error_type}"));
                }
                if let Some(code) = code {
                    parts.push(format!("code={code}"));
                }

                if parts.is_empty() {
                    Some(json_response_preview(&Value::Object(error.clone()), 500))
                } else {
                    Some(parts.join("; "))
                }
            }
            other => Some(json_response_preview(other, 500)),
        };
    }

    json.get("message")
        .and_then(non_empty_string)
        .map(|message| {
            if let Some(code) = json.get("code").and_then(non_empty_string) {
                format!("{message}; code={code}")
            } else {
                message.to_string()
            }
        })
}

pub(super) fn llm_response_shape_error(
    json: &Value,
    message: String,
    body: Option<&str>,
) -> AppError {
    let mut details = message;
    if let Some(upstream_error) = upstream_error_message(json) {
        details.push_str("；上游错误：");
        details.push_str(&upstream_error);
    }
    details.push_str("；响应片段：");
    match body {
        Some(body) => details.push_str(&response_preview(body, 800)),
        None => details.push_str(&json_response_preview(json, 800)),
    }
    internal_error(details)
}

fn upstream_error_detail(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|json| upstream_error_message(&json))
        .unwrap_or_else(|| response_preview(body, 800))
}

pub(in crate::http) fn llm_api_status_error(status: StatusCode, body: &str) -> AppError {
    let detail = upstream_error_detail(body);
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => AppError::ConfigError(format!(
            "当前 LLM 配置鉴权失败（{status}）：{detail}。请在设置页更新 API Key、Base URL 或模型名称"
        )),
        StatusCode::PAYMENT_REQUIRED => AppError::ConfigError(format!(
            "当前 LLM 配置对应账号余额不足或额度耗尽（{status}）：{detail}。请充值该供应商账号，或在设置页切换到可用的 LLM 配置"
        )),
        StatusCode::TOO_MANY_REQUESTS => AppError::LlmError(format!(
            "LLM 服务限流（{status}）：{detail}。请稍后重试或降低并发"
        )),
        status if status.as_u16() == 524 => AppError::LlmError(format!(
            "LLM 上游代理等待模型响应超时（524）：{detail}。该错误通常来自 Cloudflare 等代理的约 120 秒首包限制，请使用流式调用或切换响应更快的模型"
        )),
        StatusCode::BAD_REQUEST => {
            AppError::LlmError(format!("LLM 请求被上游拒绝（{status}）：{detail}"))
        }
        _ => AppError::LlmError(format!("LLM API 返回错误 {status}：{detail}")),
    }
}
