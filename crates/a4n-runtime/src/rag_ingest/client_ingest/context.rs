use super::super::expected_records::{ExpectedRagRecords, expected_rag_records};
use crate::{actions::access, state::AppState};
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::rag::{
    ensure_embedding_identity_matches, ensure_project_embedding_identity_matches,
};
use serde_json::Value;
use std::collections::HashMap;

pub(super) struct ClientIngestContext {
    pub(super) embedding_identity: EmbeddingModelIdentity,
    pub(super) expected: ExpectedRagRecords,
    pub(super) stored_metadata: HashMap<String, Value>,
}

pub(super) async fn load_client_ingest_context(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    rebuild: bool,
    expected_embedding_identity: Option<&a4n_api_contract::embedding::EmbeddingModelIdentity>,
) -> Result<ClientIngestContext, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    let rag = state.orchestrator();
    let embedding_identity = crate::actions::rag_search::current_embedding_identity(&rag)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(expected_embedding_identity) = expected_embedding_identity {
        ensure_embedding_identity_matches(
            &crate::dynamic_embedding::contract_identity_to_provider(expected_embedding_identity),
            &embedding_identity,
        )
        .map_err(|error| error.to_string())?;
    }
    let expected = expected_rag_records(state, project_id).await?;
    let stored_metadata = rag
        .project_rag_metadata(project_id)
        .await
        .map_err(|error| error.to_string())?;
    if !rebuild {
        ensure_project_embedding_identity_matches(&stored_metadata, &embedding_identity)
            .map_err(|error| error.to_string())?;
    }

    Ok(ClientIngestContext {
        embedding_identity,
        expected,
        stored_metadata,
    })
}
