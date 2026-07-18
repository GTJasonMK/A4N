use super::super::contracts::RagIngestTaskScope;
use super::super::expected_records::expected_rag_records;
use crate::{actions::access, state::AppState};

pub async fn expected_rag_record_total(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<usize, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    expected_rag_records(state, project_id)
        .await
        .map(|expected| expected.total())
}

pub async fn rag_ingest_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<RagIngestTaskScope, String> {
    let expected_records = expected_rag_record_total(state, user_id, project_id).await?;
    let total = rag_ingest_progress_total(expected_records)?;
    Ok(RagIngestTaskScope {
        project_id,
        expected_records,
        total,
    })
}

fn rag_ingest_progress_total(expected_records: usize) -> Result<u32, String> {
    u32::try_from(expected_records)
        .map_err(|_| format!("RAG 预期记录数超过 u32 上限：{expected_records}"))
        .map(|total| total.max(1))
}

#[cfg(test)]
mod tests {
    use super::rag_ingest_progress_total;

    #[test]
    fn rag_ingest_progress_total_keeps_zero_scope_visible() {
        assert_eq!(rag_ingest_progress_total(0).unwrap(), 1);
    }

    #[test]
    fn rag_ingest_progress_total_rejects_overflow() {
        if usize::BITS > u32::BITS {
            let expected_records = (u32::MAX as usize) + 1;
            let error = rag_ingest_progress_total(expected_records)
                .expect_err("overflow must not be silently capped at u32::MAX");
            assert!(error.contains("超过 u32 上限"));
        }
    }
}
