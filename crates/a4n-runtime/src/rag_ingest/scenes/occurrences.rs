use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use super::names::scene_profile_names;
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{scene_occurrence_rag_record, scene_occurrence_rag_record_label};

pub(crate) async fn ingest_scene_occurrences_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profile_names = scene_profile_names(state, project_id).await?;
    let occurrences = state
        .orchestrator
        .scene_occurrences_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for occurrence in &occurrences {
        let Some(scene_name) = profile_names
            .get(&occurrence.scene_profile_id)
            .map(String::as_str)
        else {
            stats.errors.push(format!(
                "场景出场 {} 找不到 scene_profile_id={} 对应场景，跳过 RAG 入库",
                occurrence.id, occurrence.scene_profile_id
            ));
            continue;
        };
        let record = match scene_occurrence_rag_record(project_id, occurrence, scene_name) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            scene_occurrence_rag_record_label(occurrence, scene_name),
            format!("场景出场 {}", occurrence.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::SceneOccurrence,
        records,
        &mut stats,
        &mut progress,
        "场景出场",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
