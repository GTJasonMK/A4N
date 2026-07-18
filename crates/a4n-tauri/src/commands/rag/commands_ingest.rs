use crate::progress_task::{WorkflowProgressTask, WorkflowTaskContext};
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::rag_ingest::{
    self, ClientRagIngestPrepareOutput, ClientRagIngestUploadInput, RagIngestProgress,
    RagProgressReporter,
};
use tauri::{State, Window};

#[tauri::command]
pub async fn rag_ingest_all(
    window: Window,
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    task_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let scope = rag_ingest::rag_ingest_task_scope(state.inner(), user_id, project_id).await?;
    let progress_task = WorkflowProgressTask::new_with_context(
        window,
        state.inner(),
        task_id.clone(),
        TaskType::RagIngestion,
        WorkflowTaskContext {
            project_id: Some(scope.project_id),
            root_project_id: Some(scope.project_id),
            source_snapshot_id: None,
            task_scope: Some("text_analysis".to_string()),
        },
        scope.total,
        "准备同步 RAG 数据",
    );
    let cancel_token = task_id.as_ref().map(|_| progress_task.cancel_token());
    let result = {
        let report_progress = |current: u32, total: u32, message: &str| {
            progress_task.progress(current, total, message);
        };
        let reporter: RagProgressReporter<'_> = task_id
            .as_ref()
            .map(|_| &report_progress as &(dyn Fn(u32, u32, &str) + Send + Sync));
        let mut progress = RagIngestProgress::new(scope.total, reporter, cancel_token.as_ref());
        rag_ingest::rag_ingest_all(
            state.inner(),
            user_id,
            project_id,
            task_id.as_deref(),
            Some(&mut progress),
        )
        .await
    };

    progress_task.finish_result(state.inner(), &result, "rag_ingest_failed", |value| {
        completion_message("RAG", value)
    });
    result
}

#[tauri::command]
pub async fn rag_prepare_ingest_all(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<ClientRagIngestPrepareOutput, String> {
    rag_ingest::rag_prepare_ingest_all(state.inner(), user_id, project_id).await
}

#[tauri::command]
pub async fn rag_upload_ingest_embeddings(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: ClientRagIngestUploadInput,
) -> Result<serde_json::Value, String> {
    rag_ingest::rag_upload_ingest_embeddings(state.inner(), user_id, project_id, input).await
}

fn completion_message(scope: &str, value: &serde_json::Value) -> String {
    let chunk_count = value
        .get("chunk_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let added_count = value
        .get("added_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let updated_count = value
        .get("updated_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let skipped_count = value
        .get("skipped_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let deleted_count = value
        .get("deleted_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    format!(
        "{scope} 同步完成：写入 {chunk_count}，新增 {added_count}，更新 {updated_count}，跳过 {skipped_count}，删除 {deleted_count}"
    )
}
