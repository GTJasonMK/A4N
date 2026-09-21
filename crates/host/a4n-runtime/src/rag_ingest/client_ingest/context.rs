use super::super::expected_records::{ExpectedRagRecords, expected_rag_records};
use crate::{actions::access, state::AppState};
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_rag_core::contracts::sync::{
    ensure_embedding_identity_matches, ensure_project_embedding_identity_matches,
};
use serde_json::Value;
use std::collections::HashMap;

pub(super) struct ClientIngestContext {
    pub(super) embedding_identity: EmbeddingModelIdentity,
    pub(super) expected: ExpectedRagRecords,
    pub(super) stored_metadata: Option<HashMap<String, Value>>,
}

pub(super) async fn load_client_ingest_context(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    rebuild: bool,
    expected_embedding_identity: Option<&EmbeddingModelIdentity>,
) -> Result<ClientIngestContext, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(expected_embedding_identity) = expected_embedding_identity {
        ensure_embedding_identity_matches(expected_embedding_identity, &embedding_identity)
            .map_err(|error| error.to_string())?;
    }
    let expected = expected_rag_records(state, project_id).await?;
    let stored_metadata = if rebuild {
        None
    } else {
        let stored_metadata = state
            .rag_ingest()
            .project_rag_metadata(project_id)
            .await
            .map_err(|error| error.to_string())?;
        ensure_project_embedding_identity_matches(&stored_metadata, &embedding_identity)
            .map_err(|error| error.to_string())?;
        Some(stored_metadata)
    };

    Ok(ClientIngestContext {
        embedding_identity,
        expected,
        stored_metadata,
    })
}
