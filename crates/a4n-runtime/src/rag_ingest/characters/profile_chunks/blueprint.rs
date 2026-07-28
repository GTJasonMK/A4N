use super::super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, cleanup_stale_records, labeled_rag_record,
    progress_stage, required_record_dimension, sync_labeled_records_concurrent,
};
use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_orchestrator::rag::{
    blueprint_character_rag_record_label, blueprint_character_rag_records,
};

pub(crate) async fn ingest_character_records_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let mut sync = state
        .rag()
        .rag_sync_context_for_data_type(project_id, RagDataType::Character)
        .await
        .map_err(|error| error.to_string())?;
    let blueprint_chars = state
        .rag()
        .blueprint_characters_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;

    for (character_index, character) in blueprint_chars.iter().enumerate() {
        let records = match blueprint_character_rag_records(project_id, character) {
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
                format!("蓝图角色 {} RAG 分块 {chunk_id}", character.id),
            ) {
                Ok(dimension) => dimension,
                Err(error) => {
                    stats.errors.push(error);
                    continue;
                }
            };
            labeled_records.push(labeled_rag_record(
                record,
                blueprint_character_rag_record_label(character, &dimension),
                format!("蓝图角色 {} 维度 {chunk_id}", character.id),
            ));
        }
        progress_stage(
            &mut progress,
            format!(
                "正在同步角色设定 {}/{}：{}",
                character_index + 1,
                blueprint_chars.len(),
                character.name
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
                .push(format!("蓝图角色 {} 未写入任何 RAG 分块", character.id));
        }
    }
    cleanup_stale_records(&mut sync, state, &mut stats, &mut progress, "角色设定").await?;
    Ok(stats)
}
