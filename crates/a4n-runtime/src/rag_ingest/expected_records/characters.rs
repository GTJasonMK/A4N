use std::collections::HashMap;

use a4n_core::models::character::protagonist_profile;
use a4n_orchestrator::rag::{
    blueprint_character_rag_records, character_attribute_event_rag_record,
    character_state_rag_record, foreshadowing_rag_record, protagonist_rag_records,
    relationship_rag_record,
};

use super::ExpectedRagRecords;
use crate::state::AppState;

pub(super) async fn append_character_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    append_blueprint_character_records(state, project_id, expected).await?;

    let profiles = state
        .orchestrator()
        .protagonist_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    append_protagonist_profile_records(project_id, &profiles, expected)?;
    append_character_timeline_records(state, project_id, &profiles, expected).await?;

    append_foreshadowing_records(state, project_id, expected).await?;
    append_relationship_records(state, project_id, expected).await
}

pub(super) async fn append_blueprint_character_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let characters = state
        .orchestrator()
        .blueprint_characters_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    for character in &characters {
        expected.add_rag_records(blueprint_character_rag_records(project_id, character)?)?;
    }
    Ok(())
}

pub(super) async fn append_protagonist_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let profiles = state
        .orchestrator()
        .protagonist_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    append_protagonist_profile_records(project_id, &profiles, expected)
}

fn append_protagonist_profile_records(
    project_id: i64,
    profiles: &[protagonist_profile::Model],
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    for profile in profiles {
        expected.add_rag_records(protagonist_rag_records(project_id, profile)?)?;
    }
    Ok(())
}

async fn append_character_timeline_records(
    state: &AppState,
    project_id: i64,
    profiles: &[protagonist_profile::Model],
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if profile_ids.is_empty() {
        return Ok(());
    }
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();
    let states = state
        .orchestrator()
        .character_states_for_rag_ingest(&profile_ids)
        .await
        .map_err(|error| error.to_string())?;
    for state_index in &states {
        let name = profile_names.get(&state_index.profile_id).ok_or_else(|| {
            format!(
                "角色状态 {} 找不到 profile_id={} 对应角色，无法构造 RAG 预期记录",
                state_index.id, state_index.profile_id
            )
        })?;
        expected.add_rag_record(character_state_rag_record(project_id, state_index, name)?)?;
    }

    let events = state
        .orchestrator()
        .attribute_events_for_rag_ingest(&profile_ids)
        .await
        .map_err(|error| error.to_string())?;
    for event in &events {
        let name = profile_names.get(&event.profile_id).ok_or_else(|| {
            format!(
                "角色属性事件 {} 找不到 profile_id={} 对应角色，无法构造 RAG 预期记录",
                event.id, event.profile_id
            )
        })?;
        expected.add_rag_record(character_attribute_event_rag_record(
            project_id, event, name,
        )?)?;
    }
    Ok(())
}

async fn append_foreshadowing_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let items = state
        .orchestrator()
        .foreshadowing_items_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    for item in &items {
        expected.add_rag_record(foreshadowing_rag_record(project_id, item)?)?;
    }
    Ok(())
}

pub(super) async fn append_relationship_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let relationships = state
        .orchestrator()
        .blueprint_relationships_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    for relationship in &relationships {
        expected.add_rag_record(relationship_rag_record(project_id, relationship)?)?;
    }
    Ok(())
}
