use super::super::contracts::{ClientRagIngestPrepareOutput, PreparedClientRagIngestRecord};
use super::super::sync::RagRecordSyncOutcome;
use super::context::{ClientIngestContext, load_client_ingest_context};
use crate::state::AppState;
use a4n_orchestrator::rag::{classify_rag_record, root_text_family_metadata};
use a4n_rag_core::metadata::content_hash;

pub async fn rag_prepare_ingest_all(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    rebuild: bool,
) -> Result<ClientRagIngestPrepareOutput, String> {
    let trace_id = crate::sidecar_clients::rag_client_ingest_operation_trace_id(project_id)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id.clone(),
        rag_prepare_ingest_all_inner(state, user_id, project_id, rebuild, trace_id),
    )
    .await
}

async fn rag_prepare_ingest_all_inner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    rebuild: bool,
    operation_trace_id: String,
) -> Result<ClientRagIngestPrepareOutput, String> {
    let ClientIngestContext {
        embedding_identity,
        expected,
        stored_metadata,
    } = load_client_ingest_context(state, user_id, project_id, rebuild, None).await?;
    let expected_total = expected.total();

    let mut records = Vec::new();
    let mut skipped_count = 0u32;
    for mut record in expected.records {
        record.metadata = root_text_family_metadata(project_id, record.metadata)
            .map_err(|error| error.to_string())?;
        let content_hash = content_hash(&record.text).map_err(|error| error.to_string())?;
        let outcome = classify_rag_record(
            stored_metadata.get(&record.chunk_id),
            &content_hash,
            &embedding_identity,
        );
        if outcome == RagRecordSyncOutcome::Skipped {
            skipped_count = skipped_count
                .checked_add(1)
                .ok_or_else(|| "RAG 跳过记录数超过 u32 上限".to_string())?;
            continue;
        }
        records.push(PreparedClientRagIngestRecord {
            chunk_id: record.chunk_id,
            text: record.text,
            content_hash,
            outcome: match outcome {
                RagRecordSyncOutcome::Added => "added",
                RagRecordSyncOutcome::Updated => "updated",
                RagRecordSyncOutcome::Skipped => "skipped",
            }
            .to_string(),
        });
    }

    Ok(ClientRagIngestPrepareOutput {
        project_id,
        operation_trace_id,
        embedding_identity,
        records,
        expected_count: expected_total,
        skipped_count,
    })
}
