use super::ExpectedRagRecords;
use super::characters::{
    append_blueprint_character_records, append_character_records, append_protagonist_records,
    append_relationship_records,
};
use super::scenes::append_scene_records;
use super::story::{append_blueprint_records, append_chapter_records, append_outline_records};
use crate::state::AppState;

pub(crate) async fn expected_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();
    append_chapter_records(state, project_id, &mut expected).await?;
    append_outline_records(state, project_id, &mut expected).await?;
    append_blueprint_records(state, project_id, &mut expected).await?;
    append_character_records(state, project_id, &mut expected).await?;
    append_scene_records(state, project_id, &mut expected).await?;
    Ok(expected)
}

pub(crate) async fn expected_outline_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();
    append_outline_records(state, project_id, &mut expected).await?;
    Ok(expected)
}

pub(crate) async fn expected_blueprint_material_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();
    append_blueprint_records(state, project_id, &mut expected).await?;
    append_blueprint_character_records(state, project_id, &mut expected).await?;
    append_protagonist_records(state, project_id, &mut expected).await?;
    append_relationship_records(state, project_id, &mut expected).await?;
    Ok(expected)
}

pub(crate) async fn expected_protagonist_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();
    append_protagonist_records(state, project_id, &mut expected).await?;
    Ok(expected)
}

pub(crate) async fn expected_scene_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();
    append_scene_records(state, project_id, &mut expected).await?;
    Ok(expected)
}
