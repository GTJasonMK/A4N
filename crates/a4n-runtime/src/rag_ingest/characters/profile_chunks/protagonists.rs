use super::super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, cleanup_stale_records, labeled_rag_record,
    progress_stage, required_record_dimension, sync_labeled_records_concurrent,
};
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{
    RagSyncContext, protagonist_rag_record_label, protagonist_rag_records,
};

pub(crate) async fn ingest_protagonists_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let mut sync =
        RagSyncContext::for_data_type(state.orchestrator(), project_id, RagDataType::Protagonist)
            .await
            .map_err(|error| error.to_string())?;
    let profiles = state
        .orchestrator()
        .protagonist_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    for (profile_index, profile) in profiles.iter().enumerate() {
        let records = match protagonist_rag_records(project_id, profile) {
            Ok(records) => records,
            Err(error) => {
                stats.errors.push(error);
                continue;
            }
        };

        let mut labeled_records = Vec::new();
        for record in records {
            let chunk_id = record.chunk_id.clone();
            let dimension = match required_record_dimension(
                &record,
                format!("主角档案 {} RAG 分块 {chunk_id}", profile.id),
            ) {
                Ok(dimension) => dimension,
                Err(error) => {
                    stats.errors.push(error);
                    continue;
                }
            };
            labeled_records.push(labeled_rag_record(
                record,
                protagonist_rag_record_label(profile, &dimension),
                format!("主角档案 {} 维度 {chunk_id}", profile.id),
            ));
        }
        progress_stage(
            &mut progress,
            format!(
                "正在同步主角档案 {}/{}：{}",
                profile_index + 1,
                profiles.len(),
                profile.name
            ),
        )?;
        let processed_chunks = sync_labeled_records_concurrent(
            &mut sync,
            state,
            labeled_records,
            &mut stats,
            &mut progress,
        )
        .await?;
        if processed_chunks > 0 {
            stats.item_count += 1;
        } else {
            stats
                .errors
                .push(format!("主角档案 {} 未写入任何 RAG 分块", profile.id));
        }
    }
    cleanup_stale_records(&mut sync, state, &mut stats, &mut progress, "主角档案").await?;
    Ok(stats)
}
