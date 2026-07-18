use a4n_core::event::sse::{ProgressMeta, SseEvent};

use super::super::ServerState;

pub(in crate::routes::commands) async fn emit_workflow_progress(
    state: &ServerState,
    task_id: &str,
    current: u32,
    total: u32,
    message: &str,
) {
    emit_workflow_event(state, task_id, SseEvent::progress(current, total, message)).await;
}

pub(in crate::routes::commands) async fn emit_workflow_progress_with_meta(
    state: &ServerState,
    task_id: &str,
    current: u32,
    total: u32,
    message: &str,
    meta: Option<ProgressMeta>,
) {
    let event = match meta {
        Some(meta) => SseEvent::progress_with_meta(current, total, message, meta),
        None => SseEvent::progress(current, total, message),
    };
    emit_workflow_event(state, task_id, event).await;
}

pub(in crate::routes::commands) async fn emit_workflow_progress_bytes(
    state: &ServerState,
    task_id: &str,
    current: u64,
    total: u64,
    message: &str,
) {
    emit_workflow_event(
        state,
        task_id,
        SseEvent::progress_bytes(current, total, message),
    )
    .await;
}

pub(in crate::routes::commands) async fn emit_workflow_chunk(
    state: &ServerState,
    task_id: &str,
    content: &str,
    index: u32,
) {
    emit_workflow_event(state, task_id, SseEvent::chunk(content, index)).await;
}

pub(in crate::routes::commands) async fn emit_workflow_complete(
    state: &ServerState,
    task_id: &str,
    summary: Option<String>,
) {
    emit_workflow_event(state, task_id, SseEvent::complete(summary)).await;
}

pub(in crate::routes::commands) async fn emit_workflow_error(
    state: &ServerState,
    task_id: &str,
    code: &str,
    message: &str,
    recoverable: bool,
) {
    emit_workflow_event(state, task_id, SseEvent::error(code, message, recoverable)).await;
}

pub(in crate::routes::commands) async fn emit_workflow_event(
    state: &ServerState,
    task_id: &str,
    event: SseEvent,
) {
    let channel = SseEvent::workflow_channel(task_id);
    let payload = match serde_json::to_value(event) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!(
                task_id,
                channel,
                error = %error,
                "workflow SSE 事件序列化失败"
            );
            return;
        }
    };
    state.web_events.publish(&channel, payload).await;
}
