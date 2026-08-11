use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{scene_profile_rag_record, scene_profile_rag_record_label};

pub(crate) async fn ingest_scene_profiles_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profiles = state
        .orchestrator()
        .scene_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for profile in &profiles {
        let record = match scene_profile_rag_record(project_id, profile) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            scene_profile_rag_record_label(profile),
            format!("场景档案 {}", profile.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::SceneProfile,
        records,
        &mut stats,
        &mut progress,
        "场景档案",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
