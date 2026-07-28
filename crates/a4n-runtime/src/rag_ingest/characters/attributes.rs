use std::collections::HashMap;

use a4n_core::rag::RagDataType;

use super::super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};
use crate::state::AppState;
use a4n_orchestrator::rag::character_attribute_event_rag_record;

pub(crate) async fn ingest_character_attribute_events_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profiles = state
        .rag()
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
            RagDataType::CharacterAttributeEvent,
            Vec::new(),
            &mut stats,
            &mut progress,
            "角色属性变更",
        )
        .await?;
        return Ok(stats);
    }
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();

    let events = state
        .rag()
        .attribute_events_for_rag_ingest(&profile_ids)
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for event in &events {
        let Some(name) = profile_names.get(&event.profile_id).map(String::as_str) else {
            stats.errors.push(format!(
                "角色属性事件 {} 找不到 profile_id={} 对应角色，跳过 RAG 入库",
                event.id, event.profile_id
            ));
            continue;
        };
        let record = match character_attribute_event_rag_record(project_id, event, name) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("角色属性变更：{} 第{}章", name, event.chapter_number),
            format!("角色属性事件 {}", event.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::CharacterAttributeEvent,
        records,
        &mut stats,
        &mut progress,
        "角色属性变更",
    )
    .await?;
    stats.add_item_count(processed)?;
    Ok(stats)
}
