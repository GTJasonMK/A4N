use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{
    blueprint_metadata_rag_record, blueprint_metadata_rag_record_label, blueprint_rag_record,
    blueprint_rag_record_label,
};

pub(crate) async fn ingest_blueprints_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let blueprints = state
        .rag()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for blueprint in &blueprints {
        let record = match blueprint_rag_record(project_id, blueprint) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            blueprint_rag_record_label(),
            format!("蓝图 {}", blueprint.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Blueprint,
        records,
        &mut stats,
        &mut progress,
        "蓝图",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}

pub(crate) async fn ingest_blueprint_metadata_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let blueprints = state
        .rag()
        .novel_blueprints_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for blueprint in &blueprints {
        let record = match blueprint_metadata_rag_record(project_id, blueprint) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            blueprint_metadata_rag_record_label(),
            format!("蓝图元数据 {}", blueprint.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::BlueprintMetadata,
        records,
        &mut stats,
        &mut progress,
        "蓝图元数据",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
