use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{
    PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome, RagSyncContext,
};

use super::progress::{
    ExpectedSyncStats, RagIngestProgress, progress_advance, progress_stage, sync_progress_message,
};
use crate::state::AppState;

pub(in crate::rag_ingest) async fn sync_project_records(
    state: &AppState,
    project_id: i64,
    records: Vec<RagRecord>,
    progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<ExpectedSyncStats, String> {
    let sync = RagSyncContext::for_project(state.orchestrator(), project_id)
        .await
        .map_err(|error| error.to_string())?;
    sync_records(state, sync, records, progress).await
}

pub(in crate::rag_ingest) async fn sync_data_type_records(
    state: &AppState,
    project_id: i64,
    data_types: &[RagDataType],
    records: Vec<RagRecord>,
) -> Result<ExpectedSyncStats, String> {
    let sync = RagSyncContext::for_data_types(state.orchestrator(), project_id, data_types)
        .await
        .map_err(|error| error.to_string())?;
    sync_records(state, sync, records, None).await
}

async fn sync_records(
    state: &AppState,
    mut sync: RagSyncContext,
    records: Vec<RagRecord>,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<ExpectedSyncStats, String> {
    let mut stats = ExpectedSyncStats::default();
    let mut pending = Vec::new();

    for record in records {
        let chunk_id = record.chunk_id.clone();
        progress_stage(&mut progress, format!("正在比对 RAG 记录：{chunk_id}"))?;
        let prepared = sync
            .prepare_record(record)
            .map_err(|error| format!("RAG 记录 {chunk_id} 无效：{error}"))?;
        if prepared.outcome() == RagRecordSyncOutcome::Skipped {
            stats.record_outcome(prepared.outcome())?;
            progress_advance(
                &mut progress,
                1,
                sync_progress_message(prepared.outcome(), chunk_id),
            )?;
        } else {
            pending.push(prepared);
        }
    }

    let batch_size = state.orchestrator().embedding_batch_size();
    if batch_size == 0 {
        return Err("RAG embedding provider batch_size 必须大于 0".to_string());
    }
    for batch in pending.chunks(batch_size) {
        state
            .orchestrator()
            .store_prepared_rag_records(sync.project_id(), batch)
            .await
            .map_err(|error| batch_store_error(batch, &error.to_string()))?;
        for prepared in batch {
            let outcome = prepared.outcome();
            stats.record_outcome(outcome)?;
            progress_advance(
                &mut progress,
                1,
                sync_progress_message(outcome, prepared.chunk_id()),
            )?;
        }
    }

    let deleted = sync
        .cleanup_stale(state.orchestrator())
        .await
        .map_err(|error| error.to_string())?;
    stats.add_deleted(deleted)?;
    if deleted > 0 {
        progress_stage(&mut progress, format!("已清理过期 RAG 记录 {deleted} 个"))?;
    }
    Ok(stats)
}

fn batch_store_error(batch: &[PreparedRagRecordSync], error: &str) -> String {
    let ids = batch
        .iter()
        .take(8)
        .map(PreparedRagRecordSync::chunk_id)
        .collect::<Vec<_>>()
        .join(", ");
    let remainder = batch.len().saturating_sub(8);
    if remainder == 0 {
        format!("RAG 记录批量入库失败 [{ids}]：{error}")
    } else {
        format!("RAG 记录批量入库失败 [{ids}]，另有 {remainder} 条：{error}")
    }
}
