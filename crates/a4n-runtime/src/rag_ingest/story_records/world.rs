use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record,
    required_record_metadata_str, sync_data_type_records,
};
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{
    blueprint_world_setting_rag_record_label, blueprint_world_setting_rag_records,
};

pub(crate) async fn ingest_world_setting_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let blueprints = state
        .orchestrator
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for blueprint in &blueprints {
        let world_setting_records = match blueprint_world_setting_rag_records(project_id, blueprint)
        {
            Ok(records) => records,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        for record in world_setting_records {
            let field = match required_record_metadata_str(
                &record,
                "field",
                format!("世界观 {} RAG 分块 {}", blueprint.id, record.chunk_id),
            ) {
                Ok(field) => field,
                Err(error) => {
                    stats.errors.push(error);
                    continue;
                }
            };
            records.push(labeled_rag_record(
                record,
                blueprint_world_setting_rag_record_label(&field),
                format!("世界观 {}", blueprint.id),
            ));
        }
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::WorldSetting,
        records,
        &mut stats,
        &mut progress,
        "世界观",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
