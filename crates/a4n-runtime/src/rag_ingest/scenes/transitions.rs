use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use super::names::scene_profile_names;
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{scene_transition_rag_record, scene_transition_rag_record_label};

pub(crate) async fn ingest_scene_transitions_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profile_names = scene_profile_names(state, project_id).await?;
    let transitions = state
        .orchestrator()
        .scene_transitions_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    let mut records = Vec::new();
    for transition in &transitions {
        let Some(to_scene_name) = profile_names
            .get(&transition.to_scene_profile_id)
            .map(String::as_str)
        else {
            stats.errors.push(format!(
                "场景位移 {} 找不到 to_scene_profile_id={} 对应场景，跳过 RAG 入库",
                transition.id, transition.to_scene_profile_id
            ));
            continue;
        };
        let from_scene_name = transition
            .from_scene_profile_id
            .and_then(|id| profile_names.get(&id))
            .map(String::as_str);
        let record = match scene_transition_rag_record(
            project_id,
            transition,
            from_scene_name,
            to_scene_name,
        ) {
            Ok(record) => record,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            scene_transition_rag_record_label(transition, to_scene_name),
            format!("场景位移 {}", transition.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::SceneTransition,
        records,
        &mut stats,
        &mut progress,
        "场景位移",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
