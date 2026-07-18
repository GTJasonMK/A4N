use std::sync::Arc;

use a4n_core::event::channel::TaskHandle;
use a4n_runtime::host_api::embedding_model as embedding_model_actions;

use super::super::ServerState;
use super::events::emit_workflow_progress_bytes;

pub(in crate::routes::commands) fn spawn_embedding_download_progress_bridge(
    state: &ServerState,
    task_id: &str,
    task: Option<Arc<TaskHandle>>,
) -> tokio::sync::mpsc::UnboundedSender<embedding_model_actions::DownloadProgress> {
    let (tx, mut rx) =
        tokio::sync::mpsc::unbounded_channel::<embedding_model_actions::DownloadProgress>();
    let state = state.clone();
    let task_id = task_id.to_string();
    let scope = embedding_model_actions::embedding_model_download_task_scope();
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if let Some(handle) = task.as_ref() {
                if progress.total_bytes > 0 {
                    let current = (u128::from(progress.downloaded_bytes.min(progress.total_bytes))
                        * u128::from(scope.total))
                        / u128::from(progress.total_bytes);
                    let current = u32::try_from(current)
                        .expect("scaled download progress is bounded by scope.total");
                    handle.update_progress(current, scope.total, &progress.message);
                }
            }
            emit_workflow_progress_bytes(
                &state,
                &task_id,
                progress.downloaded_bytes,
                progress.total_bytes,
                &progress.message,
            )
            .await;
        }
    });
    tx
}
