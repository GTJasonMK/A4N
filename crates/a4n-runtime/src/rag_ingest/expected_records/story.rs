use super::ExpectedRagRecords;
use crate::state::AppState;
use a4n_orchestrator::rag::{
    blueprint_metadata_rag_record, blueprint_rag_record, blueprint_world_setting_rag_records,
    outline_rag_record, part_outline_rag_record,
};

pub(super) async fn append_chapter_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let chapters = state
        .orchestrator()
        .chapters_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for chapter in &chapters {
        if !chapter.has_available_content() {
            continue;
        }
        let records = state
            .orchestrator()
            .chapter_rag_records(chapter, crate::rag_ingest::chapter_rag_chunking_mode())
            .await?;
        expected.add_rag_records(records)?;
    }
    Ok(())
}

pub(super) async fn append_outline_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let outlines = state
        .orchestrator()
        .chapter_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for outline in &outlines {
        let record = outline_rag_record(project_id, outline)?;
        expected.add_rag_record(record)?;
    }

    let parts = state
        .orchestrator()
        .part_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for part in &parts {
        let record = part_outline_rag_record(project_id, part)?;
        expected.add_rag_record(record)?;
    }
    Ok(())
}

pub(super) async fn append_blueprint_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let blueprints = state
        .orchestrator()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for blueprint in &blueprints {
        let record = blueprint_rag_record(project_id, blueprint)?;
        expected.add_rag_record(record)?;

        let record = blueprint_metadata_rag_record(project_id, blueprint)?;
        expected.add_rag_record(record)?;

        let records = blueprint_world_setting_rag_records(project_id, blueprint)?;
        expected.add_rag_records(records)?;
    }
    Ok(())
}
