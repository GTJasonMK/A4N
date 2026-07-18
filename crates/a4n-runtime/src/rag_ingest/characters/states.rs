use std::collections::HashMap;

use a4n_core::rag::RagDataType;

use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;
use a4n_orchestrator::rag::character_state_rag_record;

pub(crate) async fn ingest_character_states_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profiles = state
        .orchestrator
        .protagonist_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if profile_ids.is_empty() {
        sync_data_type_records(
            state,
            project_id,
            RagDataType::CharacterState,
            Vec::new(),
            &mut stats,
            &mut progress,
            "角色状态",
        )
        .await?;
        return Ok(stats);
    }
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();

    let states = state
        .orchestrator
        .character_states_for_rag_ingest(&profile_ids)
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for state_index in &states {
        let Some(name) = profile_names
            .get(&state_index.profile_id)
            .map(String::as_str)
        else {
            stats.errors.push(format!(
                "角色状态 {} 找不到 profile_id={} 对应角色，跳过 RAG 入库",
                state_index.id, state_index.profile_id
            ));
            continue;
        };
        let record = match character_state_rag_record(project_id, state_index, name) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("角色状态：{} 第{}章", name, state_index.chapter_number),
            format!("角色状态 {}", state_index.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::CharacterState,
        records,
        &mut stats,
        &mut progress,
        "角色状态",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
