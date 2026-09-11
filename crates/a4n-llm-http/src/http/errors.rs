use a4n_error::AppError;
use reqwest::{Client, header::CONTENT_ENCODING};
use std::error::Error;
use std::time::Duration;

pub(super) const MIN_CONNECT_TIMEOUT_SECS: u64 = 10;
pub(super) const MAX_CONNECT_TIMEOUT_SECS: u64 = 60;

pub(super) fn internal_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

pub(super) fn request_error(
    label: &str,
    error: reqwest::Error,
    timeout_secs: u64,
    connect_timeout_secs: u64,
    elapsed: Duration,
) -> AppError {
    if error.is_timeout() {
        let elapsed = elapsed_seconds_text(elapsed);
        if error.is_connect() {
            return AppError::Timeout(format!(
                "{label}连接超过 {connect_timeout_secs} 秒（实际等待约 {elapsed}；连接超时由 LLM 配置 timeout_secs={timeout_secs} 秒派生，上限 {MAX_CONNECT_TIMEOUT_SECS} 秒），请检查 LLM 服务地址、网络连通性、代理配置或本地模型服务是否可访问"
            ));
        }
        return AppError::Timeout(format!(
            "{label}等待响应超时（实际等待约 {elapsed}，LLM 配置 timeout_secs={timeout_secs} 秒；底层错误：{error}），请检查上游服务 deadline/代理超时，或增大 LLM 配置超时时间并降低本次生成的输出复杂度"
        ));
    }

    AppError::LlmError(format!("{label}请求失败: {error}"))
}

pub(super) fn stream_read_error(
    error: &reqwest::Error,
    read_timeout_secs: u64,
    model: &str,
    received_bytes: usize,
    parsed_chunks: usize,
    elapsed: Duration,
) -> AppError {
    let context = format!(
        "model={model}, elapsed={}, received_bytes={received_bytes}, parsed_chunks={parsed_chunks}",
        elapsed_seconds_text(elapsed)
    );
    if error.is_timeout() {
        return AppError::Timeout(format!(
            "LLM 流式响应在 {read_timeout_secs} 秒内没有收到新数据（{context}），请增大模型配置超时或检查模型服务是否仍在生成；底层错误：{}",
            error_chain(error)
        ));
    }

    let detail = error_chain(error);
    if error.is_decode() {
        return AppError::LlmError(format!(
            "LLM 流式响应体解码失败（{context}）：{detail}；上游模型服务或代理可能提前断开，或返回了不完整的压缩/chunked 响应"
        ));
    }

    AppError::LlmError(format!("LLM 流读取失败（{context}）：{detail}"))
}

fn error_chain(error: &dyn Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(error) = source {
        parts.push(error.to_string());
        source = error.source();
    }
    parts.join("；原因：")
}

pub(super) fn normalized_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.max(1)
}

pub(super) fn normalized_connect_timeout_secs(timeout_secs: u64) -> u64 {
    timeout_secs.clamp(MIN_CONNECT_TIMEOUT_SECS, MAX_CONNECT_TIMEOUT_SECS)
}

pub(super) fn elapsed_seconds_text(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds < 1.0 {
        format!("{seconds:.2} 秒")
    } else {
        format!("{seconds:.1} 秒")
    }
}

pub(super) fn build_http_client(timeout_secs: u64, connect_timeout_secs: u64) -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .read_timeout(Duration::from_secs(timeout_secs))
        .build()
        .expect("创建 HTTP 客户端失败")
}

pub(super) fn response_meta(resp: &reqwest::Response) -> String {
    let encoding = resp
        .headers()
        .get(CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("none");
    let content_length = resp
        .content_length()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "status={}, content_encoding={}, content_length={}",
        resp.status(),
        encoding,
        content_length
    )
}

pub(super) fn response_read_error(
    label: &str,
    error: reqwest::Error,
    timeout_secs: u64,
    meta: &str,
) -> AppError {
    if error.is_timeout() {
        return AppError::Timeout(format!(
            "{label}响应读取超过 {timeout_secs} 秒，请增大 LLM 配置超时时间；{meta}"
        ));
    }
    if error.is_decode() {
        return internal_error(format!(
            "{label}响应体解码失败: {error}; {meta}; 如果失败耗时接近 {timeout_secs} 秒，通常是模型生成过慢、服务端提前断开或压缩响应不完整"
        ));
    }

    internal_error(format!("{label}响应读取失败: {error}; {meta}"))
}

pub(super) async fn response_text(
    resp: reqwest::Response,
    label: &str,
    timeout_secs: u64,
) -> Result<String, AppError> {
    let meta = response_meta(&resp);
    resp.text()
        .await
        .map_err(|e| response_read_error(label, e, timeout_secs, &meta))
}

pub(super) fn response_preview(body: &str, max_chars: usize) -> String {
    let mut preview = body.chars().take(max_chars).collect::<String>();
    if body.chars().count() > max_chars {
        preview.push('…');
    }
    preview
}
