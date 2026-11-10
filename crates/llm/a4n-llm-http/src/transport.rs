use std::time::Duration;

use reqwest::{RequestBuilder, Response};

const CONNECT_RETRY_DELAYS: [Duration; 2] =
    [Duration::from_millis(100), Duration::from_millis(300)];

/// Only connection establishment can be replayed: no HTTP request has been sent.
pub(super) async fn send(builder: RequestBuilder) -> Result<Response, reqwest::Error> {
    let mut builder = builder;
    let mut delays = CONNECT_RETRY_DELAYS.into_iter().enumerate();
    loop {
        let replay = builder.try_clone();
        match builder.send().await {
            Err(error) if error.is_connect() && !error.is_timeout() => {
                let Some((builder_next, (retry_index, delay))) = replay.zip(delays.next()) else {
                    return Err(error);
                };
                tracing::warn!(
                    retry_attempt = retry_index + 1,
                    max_retries = CONNECT_RETRY_DELAYS.len(),
                    delay_ms = delay.as_millis() as u64,
                    "LLM 连接建立失败，尚未发送模型请求，正在有限重试"
                );
                tokio::time::sleep(delay).await;
                builder = builder_next;
            }
            result => return result,
        }
    }
}
