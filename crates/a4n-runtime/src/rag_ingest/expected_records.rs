mod contracts;
pub(crate) use self::contracts::ExpectedRagRecords;

use crate::state::AppState;

async fn expected_for_set(
    state: &AppState,
    project_id: i64,
    set: a4n_rag_contract::RagRecordSet,
) -> Result<ExpectedRagRecords, String> {
    let records = state
        .rag_ingest()
        .expected_records(
            project_id,
            set,
            crate::rag_ingest::chapter_rag_chunking_mode(),
        )
        .await
        .map_err(|error| error.to_string())?;
    let mut expected = ExpectedRagRecords::default();
    expected.add_rag_records(records)?;
    Ok(expected)
}

pub(crate) async fn expected_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    expected_for_set(state, project_id, a4n_rag_contract::RagRecordSet::All).await
}

pub(crate) async fn expected_outline_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    expected_for_set(state, project_id, a4n_rag_contract::RagRecordSet::Outline).await
}

pub(crate) async fn expected_blueprint_material_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    expected_for_set(
        state,
        project_id,
        a4n_rag_contract::RagRecordSet::BlueprintMaterial,
    )
    .await
}

pub(crate) async fn expected_protagonist_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    expected_for_set(
        state,
        project_id,
        a4n_rag_contract::RagRecordSet::Protagonist,
    )
    .await
}

pub(crate) async fn expected_scene_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    expected_for_set(state, project_id, a4n_rag_contract::RagRecordSet::Scene).await
}
