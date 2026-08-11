use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{
    outline_rag_record, outline_rag_record_label, part_outline_rag_record,
    part_outline_rag_record_label,
};

pub(crate) async fn ingest_outlines_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let outlines = state
        .orchestrator()
        .chapter_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for outline in &outlines {
        let record = match outline_rag_record(project_id, outline) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            outline_rag_record_label(outline),
            format!("大纲 {}", outline.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Outline,
        records,
        &mut stats,
        &mut progress,
        "章节大纲",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}

pub(crate) async fn ingest_part_outlines_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let parts = state
        .orchestrator()
        .part_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for part in &parts {
        let record = match part_outline_rag_record(project_id, part) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            part_outline_rag_record_label(part),
            format!("分部大纲 {}", part.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::PartOutline,
        records,
        &mut stats,
        &mut progress,
        "分部大纲",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
