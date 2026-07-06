use super::chapters::chapter_should_sync_rag_records;
use super::record_builders::{
    blueprint_character_rag_records, blueprint_metadata_rag_record, blueprint_rag_record,
    blueprint_world_setting_rag_records, character_attribute_event_rag_record,
    character_state_rag_record, foreshadowing_rag_record, outline_rag_record,
    part_outline_rag_record, protagonist_rag_records, relationship_rag_record,
    scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};
use super::sync::RagRecord;
use crate::state::AppState;
use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use a4n_core::models::character::{
    attribute_event, character_state_index, foreshadowing_index, protagonist_profile,
};
use a4n_core::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};
use a4n_core::models::scene::{scene_occurrence, scene_profile, scene_transition};
use a4n_services::rag::RagService;
use sea_orm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct ExpectedRagRecordCounts {
    pub(crate) chapters: usize,
    pub(crate) outlines: usize,
    pub(crate) part_outlines: usize,
    pub(crate) blueprints: usize,
    pub(crate) blueprint_metadata: usize,
    pub(crate) world_settings: usize,
    pub(crate) characters: usize,
    pub(crate) protagonists: usize,
    pub(crate) character_states: usize,
    pub(crate) character_attribute_events: usize,
    pub(crate) foreshadowing: usize,
    pub(crate) relationships: usize,
    pub(crate) scene_profiles: usize,
    pub(crate) scene_occurrences: usize,
    pub(crate) scene_transitions: usize,
}

impl ExpectedRagRecordCounts {
    pub(crate) fn total(&self) -> usize {
        self.chapters
            + self.outlines
            + self.part_outlines
            + self.blueprints
            + self.blueprint_metadata
            + self.world_settings
            + self.characters
            + self.protagonists
            + self.character_states
            + self.character_attribute_events
            + self.foreshadowing
            + self.relationships
            + self.scene_profiles
            + self.scene_occurrences
            + self.scene_transitions
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExpectedRagRecords {
    pub(crate) counts: ExpectedRagRecordCounts,
    pub(crate) ids: Vec<String>,
    pub(crate) hashes: HashMap<String, String>,
    pub(crate) records: Vec<RagRecord>,
}

impl ExpectedRagRecords {
    pub(crate) fn total(&self) -> usize {
        self.counts.total()
    }

    fn add_record(&mut self, chunk_id: String, text: &str) -> Result<(), String> {
        if self.hashes.contains_key(&chunk_id) {
            return Err(format!("RAG 预期记录 chunk_id 重复: {chunk_id}"));
        }
        let content_hash = RagService::content_hash(text).map_err(|e| e.to_string())?;
        self.hashes.insert(chunk_id.clone(), content_hash);
        self.ids.push(chunk_id);
        Ok(())
    }

    fn add_rag_record(&mut self, record: RagRecord) -> Result<(), String> {
        let RagRecord { chunk_id, text, .. } = &record;
        self.add_record(chunk_id.clone(), text)
            .map(|_| self.records.push(record))
    }

    fn add_rag_records(&mut self, records: Vec<RagRecord>) -> Result<usize, String> {
        let count = records.len();
        for record in records {
            self.add_rag_record(record)?;
        }
        Ok(count)
    }
}

pub(crate) async fn expected_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<ExpectedRagRecords, String> {
    let mut expected = ExpectedRagRecords::default();

    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for chapter in &chapters {
        if !chapter_should_sync_rag_records(chapter) {
            continue;
        }
        let records =
            crate::workflow::chapter_rag_records(chapter, state.embedding.as_ref()).await?;
        expected.counts.chapters += expected.add_rag_records(records)?;
    }

    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for outline in &outlines {
        if let Ok(record) = outline_rag_record(project_id, outline) {
            expected.counts.outlines += 1;
            expected.add_rag_record(record)?;
        }
    }

    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for part in &parts {
        if let Ok(record) = part_outline_rag_record(project_id, part) {
            expected.counts.part_outlines += 1;
            expected.add_rag_record(record)?;
        }
    }

    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for blueprint in &blueprints {
        if let Ok(record) = blueprint_rag_record(project_id, blueprint) {
            expected.counts.blueprints += 1;
            expected.add_rag_record(record)?;
        }
        if let Ok(record) = blueprint_metadata_rag_record(project_id, blueprint) {
            expected.counts.blueprint_metadata += 1;
            expected.add_rag_record(record)?;
        }
        if let Ok(records) = blueprint_world_setting_rag_records(project_id, blueprint) {
            expected.counts.world_settings += expected.add_rag_records(records)?;
        }
    }

    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for character in &characters {
        if let Ok(records) = blueprint_character_rag_records(project_id, character) {
            expected.counts.characters += expected.add_rag_records(records)?;
        }
    }

    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for profile in &profiles {
        if let Ok(records) = protagonist_rag_records(project_id, profile) {
            expected.counts.protagonists += expected.add_rag_records(records)?;
        }
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
        let states = character_state_index::Entity::find()
            .filter(character_state_index::Column::ProfileId.is_in(profile_ids.clone()))
            .all(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        for state_index in &states {
            let Some(name) = profile_names.get(&state_index.profile_id) else {
                continue;
            };
            if let Ok(record) = character_state_rag_record(project_id, state_index, name) {
                expected.counts.character_states += 1;
                expected.add_rag_record(record)?;
            }
        }

        let events = attribute_event::Entity::find()
            .filter(attribute_event::Column::ProfileId.is_in(profile_ids))
            .all(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        for event in &events {
            let Some(name) = profile_names.get(&event.profile_id) else {
                continue;
            };
            if let Ok(record) = character_attribute_event_rag_record(project_id, event, name) {
                expected.counts.character_attribute_events += 1;
                expected.add_rag_record(record)?;
            }
        }
    }

    let foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for item in &foreshadowing {
        if let Ok(record) = foreshadowing_rag_record(project_id, item) {
            expected.counts.foreshadowing += 1;
            expected.add_rag_record(record)?;
        }
    }

    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for relationship in &relationships {
        if let Ok(record) = relationship_rag_record(project_id, relationship) {
            expected.counts.relationships += 1;
            expected.add_rag_record(record)?;
        }
    }

    let scene_profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let scene_names = scene_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();
    for profile in &scene_profiles {
        if let Ok(record) = scene_profile_rag_record(project_id, profile) {
            expected.counts.scene_profiles += 1;
            expected.add_rag_record(record)?;
        }
    }

    let scene_occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for occurrence in &scene_occurrences {
        let Some(scene_name) = scene_names.get(&occurrence.scene_profile_id) else {
            continue;
        };
        if let Ok(record) = scene_occurrence_rag_record(project_id, occurrence, scene_name) {
            expected.counts.scene_occurrences += 1;
            expected.add_rag_record(record)?;
        }
    }

    let scene_transitions = scene_transition::Entity::find()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    for transition in &scene_transitions {
        let Some(to_scene_name) = scene_names.get(&transition.to_scene_profile_id) else {
            continue;
        };
        let from_scene_name = transition
            .from_scene_profile_id
            .and_then(|id| scene_names.get(&id))
            .map(String::as_str);
        if let Ok(record) =
            scene_transition_rag_record(project_id, transition, from_scene_name, to_scene_name)
        {
            expected.counts.scene_transitions += 1;
            expected.add_rag_record(record)?;
        }
    }

    Ok(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_rag_records_reject_duplicate_chunk_ids_and_store_hashes() {
        let mut expected = ExpectedRagRecords::default();

        expected
            .add_record("chunk:1".to_string(), "正文")
            .expect("first record");
        let err = expected
            .add_record("chunk:1".to_string(), "另一段正文")
            .expect_err("duplicate chunk id should reject");

        assert!(err.contains("chunk_id 重复"));
        assert_eq!(expected.ids, vec!["chunk:1".to_string()]);
        assert_eq!(expected.hashes.len(), 1);
        let expected_hash = RagService::content_hash("正文").expect("hash");
        assert_eq!(
            expected.hashes.get("chunk:1").map(String::as_str),
            Some(expected_hash.as_str())
        );
    }
}
