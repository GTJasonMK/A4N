use std::collections::HashSet;

use a4n_domain::rag::RagDataType;
use a4n_rag_core::{
    PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome, metadata::content_hash,
    sync::classify_rag_record,
};

use super::progress::{ExpectedSyncStats, RagIngestProgress};
use crate::state::AppState;

pub(in crate::rag_ingest) async fn sync_project_records(
    state: &AppState,
    project_id: i64,
    records: Vec<RagRecord>,
    progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<ExpectedSyncStats, String> {
    let preparation = state
        .rag_ingest()
        .prepare_rag_sync(project_id, None)
        .await
        .map_err(|error| error.to_string())?;
    sync_records(state, project_id, preparation, records, progress).await
}

pub(in crate::rag_ingest) async fn sync_data_type_records(
    state: &AppState,
    project_id: i64,
    data_types: &[RagDataType],
    records: Vec<RagRecord>,
) -> Result<ExpectedSyncStats, String> {
    let preparation = state
        .rag_ingest()
        .prepare_rag_sync(project_id, Some(data_types.to_vec()))
        .await
        .map_err(|error| error.to_string())?;
    sync_records(state, project_id, preparation, records, None).await
}

async fn sync_records(
    state: &AppState,
    project_id: i64,
    preparation: a4n_rag_contract::RagIngestPreparation,
    records: Vec<RagRecord>,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<ExpectedSyncStats, String> {
    let mut stats = ExpectedSyncStats::default();
    let mut pending = Vec::new();
    let mut stale_ids = preparation.stale_ids.into_iter().collect::<HashSet<_>>();

    for record in records {
        let chunk_id = record.chunk_id.clone();
        if let Some(progress) = progress.as_deref_mut() {
            progress.stage(format!("正在比对 RAG 记录：{chunk_id}"))?;
        }
        let content_hash = content_hash(&record.text).map_err(|error| error.to_string())?;
        let outcome = classify_rag_record(
            preparation.stored_metadata.get(&record.chunk_id),
            &content_hash,
            &preparation.embedding_identity,
        );
        stale_ids.remove(&record.chunk_id);
        if outcome == RagRecordSyncOutcome::Skipped {
            stats.record_outcome(outcome)?;
            if let Some(progress) = progress.as_deref_mut() {
                progress.advance(sync_progress_message(outcome, chunk_id))?;
            }
        } else {
            pending.push(PreparedRagRecordSync::new(record, outcome));
        }
    }

    let batch_size = state.rag_ingest().embedding_batch_size();
    if batch_size == 0 {
        return Err("RAG embedding provider batch_size 必须大于 0".to_string());
    }
    let mut pending = pending.into_iter();
    loop {
        let batch = pending.by_ref().take(batch_size).collect::<Vec<_>>();
        if batch.is_empty() {
            break;
        }
        let batch_status = batch
            .iter()
            .map(|prepared| (prepared.outcome(), prepared.chunk_id().to_string()))
            .collect::<Vec<_>>();
        state
            .rag_ingest()
            .store_prepared_rag_records(project_id, batch)
            .await
            .map_err(|error| batch_store_error(&batch_status, &error.to_string()))?;
        for (outcome, chunk_id) in batch_status {
            stats.record_outcome(outcome)?;
            if let Some(progress) = progress.as_deref_mut() {
                progress.advance(sync_progress_message(outcome, chunk_id))?;
            }
        }
    }

    let mut stale_ids = stale_ids.into_iter().collect::<Vec<_>>();
    stale_ids.sort();
    let deleted = state
        .rag_ingest()
        .cleanup_stale_rag_records(project_id, stale_ids)
        .await
        .map_err(|error| error.to_string())?;
    stats.add_deleted(deleted)?;
    if deleted > 0
        && let Some(progress) = progress
    {
        progress.stage(format!("已清理过期 RAG 记录 {deleted} 个"))?;
    }
    Ok(stats)
}

fn sync_progress_message(outcome: RagRecordSyncOutcome, label: impl AsRef<str>) -> String {
    let label = label.as_ref();
    match outcome {
        RagRecordSyncOutcome::Added => format!("已新增{label}"),
        RagRecordSyncOutcome::Updated => format!("已更新{label}"),
        RagRecordSyncOutcome::Skipped => format!("已跳过未变化：{label}"),
    }
}

fn batch_store_error(batch: &[(RagRecordSyncOutcome, String)], error: &str) -> String {
    let ids = batch
        .iter()
        .take(8)
        .map(|(_, chunk_id)| chunk_id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let remainder = batch.len().saturating_sub(8);
    if remainder == 0 {
        format!("RAG 记录批量入库失败 [{ids}]：{error}")
    } else {
        format!("RAG 记录批量入库失败 [{ids}]，另有 {remainder} 条：{error}")
    }
}
