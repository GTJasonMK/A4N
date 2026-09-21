//! 内部事件日志订阅。

pub(super) fn spawn_event_logger(state: &a4n_runtime::AppState) {
    let mut rx = state.subscribe_internal_events();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    tracing::info!(target: "a4n::event", event = ?event, "内部事件");
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(target: "a4n::event", skipped, "事件订阅滞后，丢弃部分事件");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!(target: "a4n::event", "事件总线已关闭，订阅 task 退出");
                    break;
                }
            }
        }
    });
}
