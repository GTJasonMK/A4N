use super::super::expected_records::{
    expected_blueprint_material_rag_records, expected_outline_rag_records,
    expected_protagonist_rag_records,
};
use super::super::sync::sync_data_type_records;
use crate::state::AppState;
use a4n_core::rag::RagDataType;

const OUTLINE_RAG_DATA_TYPES: &[RagDataType] = &[RagDataType::Outline, RagDataType::PartOutline];
const BLUEPRINT_RAG_DATA_TYPES: &[RagDataType] = &[
    RagDataType::Blueprint,
    RagDataType::BlueprintMetadata,
    RagDataType::WorldSetting,
    RagDataType::Character,
    RagDataType::Protagonist,
    RagDataType::Relationship,
];

pub(crate) async fn sync_outline_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    let expected = expected_outline_rag_records(state, project_id).await?;
    sync_data_type_records(state, project_id, OUTLINE_RAG_DATA_TYPES, expected.records)
        .await
        .map(|_| ())
}

pub(crate) async fn sync_blueprint_material_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    let expected = expected_blueprint_material_rag_records(state, project_id).await?;
    sync_data_type_records(
        state,
        project_id,
        BLUEPRINT_RAG_DATA_TYPES,
        expected.records,
    )
    .await
    .map(|_| ())
}

pub(crate) async fn sync_protagonist_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    let expected = expected_protagonist_rag_records(state, project_id).await?;
    sync_data_type_records(
        state,
        project_id,
        &[RagDataType::Protagonist],
        expected.records,
    )
    .await
    .map(|_| ())
}
