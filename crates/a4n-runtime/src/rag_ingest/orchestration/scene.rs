use super::super::contracts::SceneRagSyncTraceSource;
use super::super::expected_records::expected_scene_rag_records;
use super::super::sync::sync_data_type_records;
use crate::state::AppState;
use a4n_core::rag::RagDataType;

const SCENE_RAG_DATA_TYPES: &[RagDataType] = &[
    RagDataType::SceneProfile,
    RagDataType::SceneOccurrence,
    RagDataType::SceneTransition,
];

pub async fn sync_scene_rag_records(
    state: &AppState,
    project_id: i64,
    trace_source: SceneRagSyncTraceSource,
) -> Result<(), String> {
    let trace_id =
        crate::sidecar_clients::scene_rag_sync_operation_trace_id(project_id, trace_source)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        async move {
            let expected = expected_scene_rag_records(state, project_id).await?;
            sync_data_type_records(
                state,
                project_id,
                SCENE_RAG_DATA_TYPES,
                expected.into_records(),
            )
            .await
            .map(|_| ())
        },
    )
    .await
}
