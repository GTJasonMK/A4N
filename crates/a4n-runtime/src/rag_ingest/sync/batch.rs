use a4n_core::rag::RagDataType;
use futures::stream::{self, StreamExt};
use serde_json::Value;

use super::contracts::{LabeledRagRecord, PreparedLabeledRagRecord};
use super::progress::{
    ExpectedSyncStats, RagIngestProgress, cleanup_stale_records, progress_advance, progress_stage,
    sync_progress_message,
};
use crate::state::AppState;
use a4n_orchestrator::rag::{RagRecord, RagRecordSyncOutcome, RagSyncContext};

const RAG_SYNC_MAX_CONCURRENCY: usize = 4;

pub(in crate::rag_ingest) fn labeled_rag_record(
    record: RagRecord,
    label: impl Into<String>,
    error_context: impl Into<String>,
) -> LabeledRagRecord {
    LabeledRagRecord {
        record,
        label: label.into(),
        error_context: error_context.into(),
    }
}

pub(in crate::rag_ingest) fn required_record_metadata_str(
    record: &RagRecord,
    key: &str,
    context: impl AsRef<str>,
) -> Result<String, String> {
    record
        .metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{} 缺少 {key} 元数据", context.as_ref()))
}

pub(in crate::rag_ingest) fn required_record_dimension(
    record: &RagRecord,
    context: impl AsRef<str>,
) -> Result<String, String> {
    required_record_metadata_str(record, "dimension", context)
}

pub(in crate::rag_ingest) async fn sync_labeled_records_concurrent(
    sync: &mut RagSyncContext,
    state: &AppState,
    records: Vec<LabeledRagRecord>,
    stats: &mut ExpectedSyncStats,
    progress: &mut Option<&mut RagIngestProgress<'_>>,
) -> Result<u32, String> {
    let mut processed = 0u32;
    let mut pending = Vec::new();

    for item in records {
        progress_stage(progress, format!("正在比对 RAG 记录：{}", item.label))?;
        let prepared = match sync.prepare_record(item.record) {
            Ok(prepared) => prepared,
            Err(err) => {
                stats
                    .errors
                    .push(format!("{} 入库失败: {err}", item.error_context));
                progress_advance(
                    progress,
                    1,
                    format!("{}同步失败，继续处理后续记录", item.label),
                )?;
                continue;
            }
        };

        if prepared.outcome() == RagRecordSyncOutcome::Skipped {
            processed = processed
                .checked_add(1)
                .ok_or_else(|| "RAG 已处理记录数超过 u32 上限".to_string())?;
            stats.record_outcome(prepared.outcome())?;
            progress_advance(
                progress,
                1,
                sync_progress_message(prepared.outcome(), item.label),
            )?;
            continue;
        }

        pending.push(PreparedLabeledRagRecord {
            prepared,
            label: item.label,
            error_context: item.error_context,
        });
    }

    let project_id = sync.project_id();
    let batch_size = state.rag().embedding_batch_size().max(1);
    let batches = pending
        .chunks(batch_size)
        .map(<[PreparedLabeledRagRecord]>::to_vec)
        .collect::<Vec<_>>();
    let mut pending_stream = stream::iter(batches.into_iter().map(|batch| async move {
        let result = state
            .rag()
            .store_prepared_rag_records(project_id, batch.iter().map(|item| &item.prepared))
            .await
            .map_err(|error| error.to_string());
        (result, batch)
    }))
    .buffer_unordered(RAG_SYNC_MAX_CONCURRENCY);

    while let Some((result, batch)) = pending_stream.next().await {
        match result {
            Ok(_) => {
                for item in batch {
                    let outcome = item.prepared.outcome();
                    processed = processed
                        .checked_add(1)
                        .ok_or_else(|| "RAG 已处理记录数超过 u32 上限".to_string())?;
                    stats.record_outcome(outcome)?;
                    progress_advance(progress, 1, sync_progress_message(outcome, item.label))?;
                }
            }
            Err(err) => {
                for item in batch {
                    stats
                        .errors
                        .push(format!("{} 入库失败: {err}", item.error_context));
                    progress_advance(
                        progress,
                        1,
                        format!("{}同步失败，继续处理后续记录", item.label),
                    )?;
                }
            }
        }
    }

    Ok(processed)
}

pub(in crate::rag_ingest) async fn sync_data_type_records(
    state: &AppState,
    project_id: i64,
    data_type: RagDataType,
    records: Vec<LabeledRagRecord>,
    stats: &mut ExpectedSyncStats,
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    cleanup_label: &str,
) -> Result<u32, String> {
    let mut sync = state
        .rag()
        .rag_sync_context_for_data_type(project_id, data_type)
        .await
        .map_err(|error| error.to_string())?;
    let processed =
        sync_labeled_records_concurrent(&mut sync, state, records, stats, progress).await?;
    cleanup_stale_records(&mut sync, state, stats, progress, cleanup_label).await?;
    Ok(processed)
}
