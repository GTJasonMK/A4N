use std::collections::HashMap;

use super::ExpectedRagRecords;
use crate::state::AppState;
use a4n_orchestrator::rag::{
    blueprint_character_rag_records, character_attribute_event_rag_record,
    character_state_rag_record, foreshadowing_rag_record, protagonist_rag_records,
    relationship_rag_record,
};

pub(super) async fn append_character_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let characters = state
        .orchestrator
        .blueprint_characters_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for character in &characters {
        let records = blueprint_character_rag_records(project_id, character)?;
        expected.counts.characters += expected.add_rag_records(records)?;
    }

    let profiles = state
        .orchestrator
        .protagonist_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for profile in &profiles {
        let records = protagonist_rag_records(project_id, profile)?;
        expected.counts.protagonists += expected.add_rag_records(records)?;
    }

    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();
    if !profile_ids.is_empty() {
        let states = state
            .orchestrator
            .character_states_for_rag_ingest(&profile_ids)
            .await
            .map_err(|e| e.to_string())?;
        for state_index in &states {
            let name = profile_names.get(&state_index.profile_id).ok_or_else(|| {
                format!(
                    "角色状态 {} 找不到 profile_id={} 对应角色，无法构造 RAG 预期记录",
                    state_index.id, state_index.profile_id
                )
            })?;
            let record = character_state_rag_record(project_id, state_index, name)?;
            expected.counts.character_states += 1;
            expected.add_rag_record(record)?;
        }

        let events = state
            .orchestrator
            .attribute_events_for_rag_ingest(&profile_ids)
            .await
            .map_err(|e| e.to_string())?;
        for event in &events {
            let name = profile_names.get(&event.profile_id).ok_or_else(|| {
                format!(
                    "角色属性事件 {} 找不到 profile_id={} 对应角色，无法构造 RAG 预期记录",
                    event.id, event.profile_id
                )
            })?;
            let record = character_attribute_event_rag_record(project_id, event, name)?;
            expected.counts.character_attribute_events += 1;
            expected.add_rag_record(record)?;
        }
    }

    let foreshadowing = state
        .orchestrator
        .foreshadowing_items_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for item in &foreshadowing {
        let record = foreshadowing_rag_record(project_id, item)?;
        expected.counts.foreshadowing += 1;
        expected.add_rag_record(record)?;
    }

    let relationships = state
        .orchestrator
        .blueprint_relationships_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for relationship in &relationships {
        let record = relationship_rag_record(project_id, relationship)?;
        expected.counts.relationships += 1;
        expected.add_rag_record(record)?;
    }
    Ok(())
}
