use super::super::chapters::chapter_should_sync_rag_records;
use super::ExpectedRagRecords;
use crate::state::AppState;
use a4n_orchestrator::rag::{
    blueprint_metadata_rag_record, blueprint_rag_record, blueprint_world_setting_rag_records,
    outline_rag_record, part_outline_rag_record,
};

pub(super) async fn append_story_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let chapters = state
        .rag()
        .chapters_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for chapter in &chapters {
        if !chapter_should_sync_rag_records(chapter) {
            continue;
        }
        let records = state
            .rag()
            .chapter_rag_records(chapter, crate::rag_ingest::chapter_rag_chunking_mode())
            .await?;
        expected.counts.chapters += expected.add_rag_records(records)?;
    }

    let outlines = state
        .rag()
        .chapter_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for outline in &outlines {
        let record = outline_rag_record(project_id, outline)?;
        expected.counts.outlines += 1;
        expected.add_rag_record(record)?;
    }

    let parts = state
        .rag()
        .part_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for part in &parts {
        let record = part_outline_rag_record(project_id, part)?;
        expected.counts.part_outlines += 1;
        expected.add_rag_record(record)?;
    }

    let blueprints = state
        .rag()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for blueprint in &blueprints {
        let record = blueprint_rag_record(project_id, blueprint)?;
        expected.counts.blueprints += 1;
        expected.add_rag_record(record)?;

        let record = blueprint_metadata_rag_record(project_id, blueprint)?;
        expected.counts.blueprint_metadata += 1;
        expected.add_rag_record(record)?;

        let records = blueprint_world_setting_rag_records(project_id, blueprint)?;
        expected.counts.world_settings += expected.add_rag_records(records)?;
    }
    Ok(())
}
