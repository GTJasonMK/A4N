use std::collections::BTreeSet;

use a4n_db_models::models::scene::scene_profile;
use a4n_domain::profile_ownership::{PROFILE_OWNERSHIP_AI_SYNC, PROFILE_OWNERSHIP_USER};
use a4n_domain::profile_text::merge_profile_evidence_json;
use a4n_domain::scene_profile::SceneProfileRecordInput;

use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult, SceneProfileRecordDraft};

#[derive(Default)]
pub(super) struct SceneProfileDraftTracker {
    refs: BTreeSet<String>,
    normalized_names: BTreeSet<String>,
    explicit_profile_ids: BTreeSet<i64>,
}

impl SceneProfileDraftTracker {
    pub(super) fn accept(&mut self, draft: &SceneProfileRecordDraft) -> SceneProfileDbResult<()> {
        if !self.refs.insert(draft.scene_ref.clone()) {
            return Err(SceneProfileDbError::InvalidArgument(format!(
                "场景同步存在重复 scene_ref: {}",
                draft.scene_ref
            )));
        }
        if !self
            .normalized_names
            .insert(draft.input.normalized_name.clone())
        {
            return Err(SceneProfileDbError::InvalidArgument(format!(
                "场景同步存在重复场景名称: {}",
                draft.input.name
            )));
        }
        if let Some(id) = draft.scene_profile_id
            && !self.explicit_profile_ids.insert(id)
        {
            return Err(SceneProfileDbError::InvalidArgument(format!(
                "场景同步重复引用同一场景档案: {id}"
            )));
        }
        Ok(())
    }
}

pub(super) fn merge_scene_profile_record_input(
    existing: &scene_profile::Model,
    mut input: SceneProfileRecordInput,
) -> SceneProfileRecordInput {
    let range_unconfirmed = existing.ownership_kind == PROFILE_OWNERSHIP_USER
        && existing.evidence_json.as_array().is_some_and(Vec::is_empty);
    let first_seen_chapter = if range_unconfirmed {
        input.first_seen_chapter
    } else {
        existing.first_seen_chapter.min(input.first_seen_chapter)
    };
    let last_seen_chapter = if range_unconfirmed {
        input.last_seen_chapter
    } else {
        existing.last_seen_chapter.max(input.last_seen_chapter)
    };
    let evidence_json = merge_profile_evidence_json(&existing.evidence_json, &input.evidence_json);
    let aliases_json = merge_profile_evidence_json(&existing.aliases_json, &input.aliases_json);
    if existing.ownership_kind != PROFILE_OWNERSHIP_AI_SYNC {
        input = merge_user_scene_profile_fields(existing, input);
    }
    input.first_seen_chapter = first_seen_chapter;
    input.last_seen_chapter = last_seen_chapter;
    input.evidence_json = evidence_json;
    input.aliases_json = aliases_json;
    input.name = existing.name.clone();
    input.normalized_name = existing.normalized_name.clone();
    input
}

fn merge_user_scene_profile_fields(
    existing: &scene_profile::Model,
    incoming: SceneProfileRecordInput,
) -> SceneProfileRecordInput {
    let mut merged = scene_profile_record_input(existing);
    if existing.aliases_json.as_array().is_some_and(Vec::is_empty) {
        merged.aliases_json = incoming.aliases_json;
    }
    merged.scene_type = incoming.scene_type;
    fill_blank(&mut merged.importance, incoming.importance);
    if merged.parent_scene_id.is_none() {
        merged.parent_scene_id = incoming.parent_scene_id;
    }
    merged.base_description = incoming.base_description;
    merged.visual_description = incoming.visual_description;
    merged.spatial_layout = incoming.spatial_layout;
    merged.atmosphere = incoming.atmosphere;
    merged.entry_points_json = incoming.entry_points_json;
    merged.rules_json = incoming.rules_json;
    fill_blank(&mut merged.image_prompt_seed, incoming.image_prompt_seed);
    merged
}

fn fill_blank(current: &mut String, incoming: String) {
    if current.trim().is_empty() {
        *current = incoming;
    }
}

pub(super) fn scene_profile_matches_record_input(
    existing: &scene_profile::Model,
    input: &SceneProfileRecordInput,
) -> bool {
    existing.name == input.name
        && existing.normalized_name == input.normalized_name
        && existing.aliases_json == input.aliases_json
        && existing.scene_type == input.scene_type
        && existing.importance == input.importance
        && existing.parent_scene_id == input.parent_scene_id
        && existing.base_description == input.base_description
        && existing.visual_description == input.visual_description
        && existing.spatial_layout == input.spatial_layout
        && existing.atmosphere == input.atmosphere
        && existing.entry_points_json == input.entry_points_json
        && existing.rules_json == input.rules_json
        && existing.first_seen_chapter == input.first_seen_chapter
        && existing.last_seen_chapter == input.last_seen_chapter
        && existing.evidence_json == input.evidence_json
        && existing.image_prompt_seed == input.image_prompt_seed
}

fn scene_profile_record_input(existing: &scene_profile::Model) -> SceneProfileRecordInput {
    SceneProfileRecordInput {
        name: existing.name.clone(),
        normalized_name: existing.normalized_name.clone(),
        aliases_json: existing.aliases_json.clone(),
        scene_type: existing.scene_type.clone(),
        importance: existing.importance.clone(),
        parent_scene_id: existing.parent_scene_id,
        base_description: existing.base_description.clone(),
        visual_description: existing.visual_description.clone(),
        spatial_layout: existing.spatial_layout.clone(),
        atmosphere: existing.atmosphere.clone(),
        entry_points_json: existing.entry_points_json.clone(),
        rules_json: existing.rules_json.clone(),
        first_seen_chapter: existing.first_seen_chapter,
        last_seen_chapter: existing.last_seen_chapter,
        evidence_json: existing.evidence_json.clone(),
        image_prompt_seed: existing.image_prompt_seed.clone(),
    }
}
