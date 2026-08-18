use super::super::contracts::RagIngestTaskScope;
use super::super::expected_records::expected_rag_records;
use crate::{actions::access, state::AppState};

pub async fn rag_ingest_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<RagIngestTaskScope, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    let expected = expected_rag_records(state, project_id).await?;
    let total = u32::try_from(expected.total())
        .map_err(|_| format!("RAG 预期记录数超过 u32 上限：{}", expected.total()))?;
    Ok(RagIngestTaskScope {
        project_id,
        total,
        expected,
    })
}
