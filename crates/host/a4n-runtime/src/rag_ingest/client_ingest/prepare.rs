use super::super::sync::RagRecordSyncOutcome;
use super::context::{ClientIngestContext, load_client_ingest_context};
use crate::state::AppState;
use a4n_api_contract::rag::{ClientRagIngestPrepareOutput, PreparedClientRagIngestRecord};
use a4n_rag_core::{contracts::sync::classify_rag_record, metadata::content_hash};

pub async fn rag_prepare_ingest_all(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    rebuild: bool,
) -> Result<ClientRagIngestPrepareOutput, String> {
    let trace_id = crate::sidecar_clients::rag_client_ingest_operation_trace_id(project_id)?;
    let scoped_trace_id = trace_id.clone();
    crate::sidecar_clients::scope_rag_operation_trace(scoped_trace_id, async move {
        let ClientIngestContext {
            embedding_identity,
            expected,
            stored_metadata,
        } = load_client_ingest_context(state, user_id, project_id, rebuild, None).await?;

        let mut records = Vec::new();
        for record in expected.into_records() {
            let content_hash = content_hash(&record.text).map_err(|error| error.to_string())?;
            let outcome = classify_rag_record(
                stored_metadata
                    .as_ref()
                    .and_then(|stored| stored.get(&record.chunk_id)),
                &content_hash,
                &embedding_identity,
            );
            if outcome == RagRecordSyncOutcome::Skipped {
                continue;
            }
            records.push(PreparedClientRagIngestRecord {
                chunk_id: record.chunk_id,
                text: record.text,
                content_hash,
            });
        }

        Ok(ClientRagIngestPrepareOutput {
            operation_trace_id: trace_id,
            embedding_identity,
            records,
        })
    })
    .await
}
