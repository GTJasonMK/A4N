use std::collections::HashMap;

use a4n_api_contract::rag::{ClientRagIngestUploadInput, RagIngestResult};
use a4n_orchestrator::rag::{
    DeleteRagRecordsInput, RagEmbeddedInput, RagRecord, RagRecordSyncOutcome, classify_rag_record,
};
use a4n_rag_core::{metadata::content_hash, validation::validate_embedding_batch};

use super::super::sync::ExpectedSyncStats;
use super::context::{ClientIngestContext, load_client_ingest_context};
use crate::state::AppState;

struct PendingClientEmbedding {
    text: String,
    metadata: serde_json::Value,
    content_hash: String,
    outcome: RagRecordSyncOutcome,
}

pub async fn rag_upload_ingest_embeddings(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ClientRagIngestUploadInput,
) -> Result<RagIngestResult, String> {
    let trace_id = crate::sidecar_clients::validate_rag_client_ingest_operation_trace_id(
        project_id,
        &input.operation_trace_id,
    )?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        rag_upload_ingest_embeddings_inner(state, user_id, project_id, input),
    )
    .await
}

async fn rag_upload_ingest_embeddings_inner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ClientRagIngestUploadInput,
) -> Result<RagIngestResult, String> {
    let ClientIngestContext {
        embedding_identity,
        expected,
        stored_metadata,
    } = load_client_ingest_context(
        state,
        user_id,
        project_id,
        input.rebuild,
        Some(&input.embedding_identity),
    )
    .await?;
    let (expected_ids, expected_records) = expected.into_parts();
    let mut pending_by_id = HashMap::new();
    let mut stats = ExpectedSyncStats::default();

    for record in expected_records {
        let hash = content_hash(&record.text).map_err(|error| error.to_string())?;
        let outcome = classify_rag_record(
            stored_metadata
                .as_ref()
                .and_then(|stored| stored.get(&record.chunk_id)),
            &hash,
            &embedding_identity,
        );
        if outcome == RagRecordSyncOutcome::Skipped {
            stats.record_outcome(outcome)?;
            continue;
        }
        let RagRecord {
            chunk_id,
            text,
            metadata,
        } = record;
        pending_by_id.insert(
            chunk_id,
            PendingClientEmbedding {
                text,
                metadata,
                content_hash: hash,
                outcome,
            },
        );
    }

    if input.embeddings.len() != pending_by_id.len() {
        return Err(format!(
            "RAG 上传记录数与当前 prepare 结果不一致：expected={}, actual={}",
            pending_by_id.len(),
            input.embeddings.len()
        ));
    }
    let vectors = input
        .embeddings
        .iter()
        .map(|uploaded| uploaded.embedding.as_slice())
        .collect::<Vec<_>>();
    validate_embedding_batch(&vectors, pending_by_id.len(), embedding_identity.dimension)
        .map_err(|error| error.to_string())?;

    let mut embedded_inputs = Vec::with_capacity(input.embeddings.len());
    for uploaded in input.embeddings {
        if uploaded.chunk_id.is_empty() || uploaded.chunk_id != uploaded.chunk_id.trim() {
            return Err("RAG 上传失败：chunk_id 必须是无首尾空白的非空字符串".to_string());
        }
        let pending = pending_by_id.remove(&uploaded.chunk_id).ok_or_else(|| {
            format!(
                "RAG 上传失败：chunk_id 不属于当前待上传 prepare 结果：{}",
                uploaded.chunk_id
            )
        })?;
        if uploaded.content_hash != pending.content_hash {
            return Err(format!(
                "RAG 上传失败：{} 的 canonical 文本已变化，请重新 prepare",
                uploaded.chunk_id
            ));
        }
        stats.record_outcome(pending.outcome)?;
        embedded_inputs.push(RagEmbeddedInput {
            chunk_id: uploaded.chunk_id,
            text: pending.text,
            metadata: pending.metadata,
            embedding: uploaded.embedding,
        });
    }
    if input.rebuild {
        state
            .orchestrator()
            .clear_project_rag(project_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    state
        .orchestrator()
        .store_embedded_rag_records(project_id, embedded_inputs, embedding_identity)
        .await
        .map_err(|error| error.to_string())?;

    if let Some(stored_metadata) = stored_metadata {
        let stale_ids = stored_metadata
            .keys()
            .filter(|id| !expected_ids.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        let deleted = state
            .orchestrator()
            .delete_rag_records(DeleteRagRecordsInput {
                project_id,
                ids: stale_ids,
                id_prefixes: Vec::new(),
            })
            .await
            .map_err(|error| format!("清理过期 RAG 记录失败: {error}"))?;
        stats.add_deleted(
            u32::try_from(deleted)
                .map_err(|_| format!("清理过期 RAG 记录数量超过 u32 上限：{deleted}"))?,
        )?;
    }

    Ok(stats.into_result())
}
