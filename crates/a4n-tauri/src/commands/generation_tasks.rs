//! Generation task 的 Tauri Channel transport。
//!
//! Channel 只传递同一个 session replay/live stream；权限、snapshot、sequence
//! 和终态仍由 Runtime/Orchestrator owner 负责。

use a4n_api_contract::generation_tasks::{
    GenerationSubscriptionHandle, GenerationTaskHistoryPage, GenerationTaskHistoryQuery,
    GenerationTaskSummary, HostErrorResponse, UnsubscribeGenerationTaskInput,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::generation_tasks as generation_actions;
use a4n_service_protocol::{GenerationErrorCode, GenerationTaskEvent};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{Runtime, State, Window, ipc::Channel};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
pub(crate) struct GenerationSubscriptionRegistry {
    next_id: Arc<AtomicU64>,
    entries: Arc<Mutex<HashMap<String, SubscriptionEntry>>>,
}

struct SubscriptionEntry {
    owner_window: String,
    cancel: CancellationToken,
}

impl GenerationSubscriptionRegistry {
    fn register(&self, owner_window: String, cancel: CancellationToken) -> String {
        let id = format!(
            "tauri-generation-{}",
            self.next_id.fetch_add(1, Ordering::Relaxed)
        );
        self.entries.lock().insert(
            id.clone(),
            SubscriptionEntry {
                owner_window,
                cancel,
            },
        );
        id
    }

    fn unsubscribe(&self, owner_window: &str, subscription_id: &str) -> bool {
        let mut entries = self.entries.lock();
        let Some(entry) = entries.get(subscription_id) else {
            return false;
        };
        if entry.owner_window != owner_window {
            return false;
        }
        let entry = entries
            .remove(subscription_id)
            .expect("subscription entry exists while locked");
        entry.cancel.cancel();
        true
    }

    fn finish(&self, subscription_id: &str) {
        self.entries.lock().remove(subscription_id);
    }

    pub(crate) fn cancel_window(&self, owner_window: &str) -> usize {
        let entries = {
            let mut guard = self.entries.lock();
            let ids = guard
                .iter()
                .filter(|(_, entry)| entry.owner_window == owner_window)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            ids.into_iter()
                .filter_map(|id| guard.remove(&id))
                .collect::<Vec<_>>()
        };
        let cancelled = entries.len();
        for entry in entries {
            entry.cancel.cancel();
        }
        cancelled
    }
}

/// 生产 `WindowEvent::Destroyed` 的唯一清理入口。
///
/// app 事件循环与 process 证据都必须调用本函数，禁止测试旁路另一套清理逻辑。
pub(crate) fn handle_window_destroyed(
    registry: &GenerationSubscriptionRegistry,
    window_label: &str,
) -> usize {
    let subscriptions_cancelled = registry.cancel_window(window_label);
    tracing::info!(
        window_label,
        subscriptions_cancelled,
        "generation Tauri window destroyed"
    );
    subscriptions_cancelled
}

#[tauri::command]
pub async fn subscribe_generation_task<R: Runtime>(
    window: Window<R>,
    state: State<'_, AppState>,
    registry: State<'_, GenerationSubscriptionRegistry>,
    user_id: i64,
    task_id: String,
    attempt: u32,
    after_sequence: Option<u64>,
    on_event: Channel<GenerationTaskEvent>,
) -> Result<GenerationSubscriptionHandle, HostErrorResponse> {
    if attempt == 0 {
        return Err(host_error(
            "generation_request_invalid",
            "attempt 必须大于 0",
            false,
        ));
    }
    let task = generation_actions::authorize_generation_task(&state, user_id, &task_id, attempt)
        .await
        .map_err(map_access_error)?;
    let mut subscription =
        generation_actions::subscribe_generation_task(&state, &task, after_sequence)
            .map_err(map_access_error)?;
    let initial = subscription.take_initial().ok_or_else(|| {
        host_error(
            "generation_checkpoint_unavailable",
            "generation subscription 缺少初始 replay",
            true,
        )
    })?;
    let (pending, latest_sequence) = match initial {
        generation_actions::GenerationReplay::Events {
            events,
            latest_sequence,
            ..
        } => {
            if events.is_empty()
                && task
                    .public_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.status.is_terminal())
            {
                return Err(host_error(
                    "generation_snapshot_required",
                    "终态任务没有可继续回放的事件，请先读取 snapshot",
                    false,
                ));
            }
            (events, latest_sequence)
        }
        generation_actions::GenerationReplay::SnapshotRequired { .. } => {
            return Err(host_error(
                "generation_snapshot_required",
                "replay cursor 已超出保留范围，请先读取 snapshot",
                false,
            ));
        }
    };

    let cancel = CancellationToken::new();
    let subscription_id = registry.register(window.label().to_string(), cancel.clone());
    let owner_registry = registry.inner().clone();
    let worker_id = subscription_id.clone();
    tauri::async_runtime::spawn(async move {
        pump_subscription(
            worker_id.clone(),
            subscription,
            pending,
            latest_sequence,
            on_event,
            cancel,
        )
        .await;
        owner_registry.finish(&worker_id);
    });

    Ok(GenerationSubscriptionHandle {
        subscription_id,
        task_id,
        attempt,
        subscribed_after_sequence: after_sequence,
    })
}

#[tauri::command]
pub async fn unsubscribe_generation_task<R: Runtime>(
    window: Window<R>,
    registry: State<'_, GenerationSubscriptionRegistry>,
    input: UnsubscribeGenerationTaskInput,
) -> Result<(), HostErrorResponse> {
    if registry.unsubscribe(window.label(), &input.subscription_id) {
        Ok(())
    } else {
        Err(host_error(
            "generation_task_forbidden",
            "subscription 不存在或不属于当前窗口",
            false,
        ))
    }
}

#[tauri::command]
pub async fn get_generation_task_summary(
    state: State<'_, AppState>,
    user_id: i64,
    task_id: String,
    attempt: u32,
) -> Result<a4n_api_contract::generation_tasks::GenerationTaskSummary, HostErrorResponse> {
    let task = generation_actions::authorize_generation_task(&state, user_id, &task_id, attempt)
        .await
        .map_err(map_access_error)?;
    generation_actions::generation_task_summary_for_host(&task).map_err(map_access_error)
}

#[tauri::command]
pub async fn get_generation_task_snapshot(
    state: State<'_, AppState>,
    user_id: i64,
    task_id: String,
    attempt: u32,
) -> Result<a4n_service_protocol::GenerationTaskSnapshot, HostErrorResponse> {
    let task = generation_actions::authorize_generation_task(&state, user_id, &task_id, attempt)
        .await
        .map_err(map_access_error)?;
    task.public_snapshot.ok_or_else(|| {
        if task
            .checkpoint
            .expires_at
            .is_some_and(|expires_at| expires_at <= chrono::Utc::now().naive_utc())
        {
            host_error(
                "generation_snapshot_expired",
                "generation snapshot 已过期",
                false,
            )
        } else {
            host_error(
                "generation_checkpoint_unavailable",
                "generation snapshot 当前不可用",
                true,
            )
        }
    })
}

#[tauri::command]
pub async fn list_active_generation_tasks(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: Option<i64>,
) -> Result<Vec<GenerationTaskSummary>, HostErrorResponse> {
    generation_actions::list_active_generation_tasks(&state, user_id, project_id)
        .await
        .map_err(map_access_error)
}

#[tauri::command]
pub async fn list_generation_task_history(
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerationTaskHistoryQuery,
) -> Result<GenerationTaskHistoryPage, HostErrorResponse> {
    let cursor = input
        .cursor
        .as_deref()
        .map(generation_actions::decode_generation_history_cursor)
        .transpose()
        .map_err(|error| host_error("generation_cursor_invalid", error.to_string(), false))?;
    generation_actions::list_generation_task_history(&state, user_id, input, cursor)
        .await
        .map_err(map_access_error)
}

async fn pump_subscription(
    subscription_id: String,
    mut subscription: generation_actions::GenerationSubscription,
    pending: Vec<GenerationTaskEvent>,
    mut cursor: u64,
    channel: Channel<GenerationTaskEvent>,
    cancel: CancellationToken,
) {
    for event in pending {
        if cancel.is_cancelled() {
            return;
        }
        cursor = event.sequence;
        let terminal = event.event.is_terminal();
        if channel.send(event).is_err() {
            tracing::debug!(subscription_id = %subscription_id, "generation Tauri Channel 已关闭");
            return;
        }
        if terminal {
            return;
        }
    }
    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            changed = subscription.receiver().changed() => {
                if changed.is_err() { return; }
            }
        }
        match subscription.replay_after(Some(cursor)) {
            generation_actions::GenerationReplay::Events {
                events,
                latest_sequence,
                ..
            } => {
                if events.is_empty() {
                    if latest_sequence <= cursor && subscription.snapshot().status.is_terminal() {
                        return;
                    }
                    continue;
                }
                for event in events {
                    if cancel.is_cancelled() {
                        return;
                    }
                    cursor = event.sequence;
                    let terminal = event.event.is_terminal();
                    if channel.send(event).is_err() {
                        tracing::debug!(subscription_id = %subscription_id, "generation Tauri Channel 已关闭");
                        return;
                    }
                    if terminal {
                        return;
                    }
                }
            }
            generation_actions::GenerationReplay::SnapshotRequired { .. } => {
                tracing::warn!(subscription_id = %subscription_id, cursor, "generation Tauri replay 需要 snapshot");
                return;
            }
        }
    }
}

fn host_error(
    code: &'static str,
    message: impl Into<String>,
    retryable: bool,
) -> HostErrorResponse {
    HostErrorResponse {
        code: GenerationErrorCode::registered(code)
            .expect("generation Tauri error code must exist"),
        message: message.into(),
        retryable,
        trace_id: None,
    }
}

pub(crate) fn map_access_error(
    error: generation_actions::GenerationTaskAccessError,
) -> HostErrorResponse {
    let code = error.code();
    host_error(
        code,
        error.to_string(),
        matches!(
            code,
            "generation_checkpoint_unavailable"
                | "generation_session_capacity_exceeded"
                | "generation_subscription_capacity_exceeded"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriptions_are_window_owned_and_destroy_cleanup_cancels_them() {
        let registry = GenerationSubscriptionRegistry::default();
        let main_cancel = CancellationToken::new();
        let main_id = registry.register("main".to_string(), main_cancel.clone());
        let main_second = CancellationToken::new();
        let main_second_id = registry.register("main".to_string(), main_second.clone());
        let other_cancel = CancellationToken::new();
        let other_id = registry.register("other".to_string(), other_cancel.clone());

        assert!(!registry.unsubscribe("other", &main_id));
        assert!(!main_cancel.is_cancelled());
        assert!(!main_second.is_cancelled());
        assert!(!other_cancel.is_cancelled());

        // 与 app.rs `WindowEvent::Destroyed` 使用同一入口，不经过 MockRuntime。
        assert_eq!(handle_window_destroyed(&registry, "main"), 2);
        assert!(main_cancel.is_cancelled());
        assert!(main_second.is_cancelled());
        assert!(!other_cancel.is_cancelled(), "只能清理被销毁窗口的订阅");
        assert!(!registry.unsubscribe("main", &main_id));
        assert!(!registry.unsubscribe("main", &main_second_id));

        // Destroyed 后同 label 可重新注册：重挂载不会继承已取消 token。
        let remount_cancel = CancellationToken::new();
        let remount_id = registry.register("main".to_string(), remount_cancel.clone());
        assert!(!remount_cancel.is_cancelled());
        assert!(registry.unsubscribe("main", &remount_id));
        assert!(remount_cancel.is_cancelled());

        assert!(registry.unsubscribe("other", &other_id));
        assert!(other_cancel.is_cancelled());
    }
}

#[cfg(all(test, feature = "embedded-media-providers"))]
mod process_tests;
