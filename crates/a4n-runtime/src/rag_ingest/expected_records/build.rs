use super::ExpectedRagRecords;
use super::characters::append_character_records;
use super::scenes::append_scene_records;
use super::story::append_story_records;
use crate::state::AppState;

pub(crate) async fn expected_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();
    append_story_records(state, project_id, &mut expected).await?;
    append_character_records(state, project_id, &mut expected).await?;
    append_scene_records(state, project_id, &mut expected).await?;
    Ok(expected)
}
