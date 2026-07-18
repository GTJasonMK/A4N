use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{relationship_rag_record, relationship_rag_record_label};

use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;

pub(crate) async fn ingest_relationships_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let relationships = state
        .orchestrator
        .blueprint_relationships_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    for relationship in &relationships {
        let record = match relationship_rag_record(project_id, relationship) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            relationship_rag_record_label(relationship),
            format!("角色关系 {}", relationship.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Relationship,
        records,
        &mut stats,
        &mut progress,
        "角色关系",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
