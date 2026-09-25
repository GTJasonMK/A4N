use super::super::expected_records::{
    expected_blueprint_material_rag_records, expected_outline_rag_records,
    expected_protagonist_rag_records,
};
use super::super::sync::sync_data_type_records;
use crate::state::AppState;
use a4n_domain::rag::RagDataType;

const OUTLINE_RAG_DATA_TYPES: &[RagDataType] = &[RagDataType::Outline, RagDataType::PartOutline];
const BLUEPRINT_RAG_DATA_TYPES: &[RagDataType] = &[
    RagDataType::Blueprint,
    RagDataType::BlueprintMetadata,
    RagDataType::WorldSetting,
    RagDataType::Character,
    RagDataType::Protagonist,
    RagDataType::Relationship,
];

pub(crate) async fn sync_character_continuity_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    // ponytail: scan the current project and reuse hash-based embedding skips; scope by chapter if scans become costly.
    let expected = super::super::expected_records::expected_for_set(
        state,
        project_id,
        a4n_rag_contract::RagRecordSet::CharacterContinuity,
    )
    .await?;
    sync_data_type_records(
        state,
        project_id,
        &[
            RagDataType::Character,
            RagDataType::Protagonist,
            RagDataType::CharacterState,
            RagDataType::CharacterAttributeEvent,
            RagDataType::Foreshadowing,
            RagDataType::Relationship,
        ],
        expected.into_records(),
    )
    .await
    .map(|_| ())
}

pub(crate) async fn sync_outline_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    let expected = expected_outline_rag_records(state, project_id).await?;
    sync_data_type_records(
        state,
        project_id,
        OUTLINE_RAG_DATA_TYPES,
        expected.into_records(),
    )
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
        expected.into_records(),
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
        expected.into_records(),
    )
    .await
    .map(|_| ())
}
