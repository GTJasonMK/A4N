use crate::progress_task::{WorkflowProgressTask, WorkflowTaskContext};
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::rag_ingest::{
    self, ClientRagIngestPrepareOutput, ClientRagIngestUploadInput, RagIngestProgress,
    RagProgressReporter,
};
use tauri::State;

#[tauri::command]
pub async fn rag_ingest_all(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    task_id: Option<String>,
    rebuild: Option<bool>,
) -> Result<serde_json::Value, String> {
    let scope = rag_ingest::rag_ingest_task_scope(state.inner(), user_id, project_id).await?;
    let progress_task = WorkflowProgressTask::new_with_context(
        state.inner(),
        Some(user_id),
        task_id.clone(),
        TaskKind::RagIngestion,
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
            rebuild.unwrap_or(false),
            Some(&mut progress),
        )
        .await
    };

    progress_task.remove(state.inner());
    result
}

#[tauri::command]
pub async fn rag_prepare_ingest_all(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    rebuild: Option<bool>,
) -> Result<ClientRagIngestPrepareOutput, String> {
    rag_ingest::rag_prepare_ingest_all(state.inner(), user_id, project_id, rebuild.unwrap_or(false))
        .await
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
