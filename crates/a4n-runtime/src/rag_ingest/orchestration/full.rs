use super::super::contracts::RagIngestTaskScope;
use super::super::sync::{RagIngestProgress, sync_project_records};
use crate::{actions::access, state::AppState};
use a4n_api_contract::rag::RagIngestResult;

pub async fn rag_ingest_all(
    state: &AppState,
    user_id: i64,
    scope: RagIngestTaskScope,
    task_id: Option<&str>,
    rebuild: bool,
    progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<RagIngestResult, String> {
    let trace_id =
        crate::sidecar_clients::rag_ingest_operation_trace_id(scope.project_id, task_id)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        rag_ingest_all_inner(state, user_id, scope, rebuild, progress),
    )
    .await
}

async fn rag_ingest_all_inner(
    state: &AppState,
    user_id: i64,
    scope: RagIngestTaskScope,
    rebuild: bool,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<RagIngestResult, String> {
    access::verify_project_editor(state, user_id, scope.project_id)
        .await
        .map_err(|error| error.to_string())?;
    if rebuild {
        state
            .orchestrator()
            .clear_project_rag(scope.project_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    if let Some(progress) = progress.as_deref_mut() {
        progress.stage("正在检查已同步 RAG 数据")?;
    }
    sync_project_records(state, scope.project_id, scope.expected.records, progress)
        .await
        .map(|stats| stats.into_result())
}
