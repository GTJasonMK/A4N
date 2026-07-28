use a4n_core::rag::RagDataType;

use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;
use a4n_orchestrator::rag::foreshadowing_rag_record;

pub(crate) async fn ingest_foreshadowing_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let items = state
        .rag()
        .foreshadowing_items_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for item in &items {
        let record = match foreshadowing_rag_record(project_id, item) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("伏笔：第{}章", item.setup_chapter),
            format!("伏笔 {}", item.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Foreshadowing,
        records,
        &mut stats,
        &mut progress,
        "伏笔",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
