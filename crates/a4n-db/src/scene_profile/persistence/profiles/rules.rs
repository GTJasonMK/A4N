use std::collections::BTreeSet;

use a4n_core::models::scene::scene_profile;
use serde_json::{Value, json};

use crate::scene_profile::{
    SceneProfileDbError, SceneProfileDbResult, SceneProfileRecordDraft, SceneProfileRecordInput,
};

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
    input.first_seen_chapter = existing.first_seen_chapter.min(input.first_seen_chapter);
    input.last_seen_chapter = existing.last_seen_chapter.max(input.last_seen_chapter);
    input.evidence_json = merge_text_set_json_values(&existing.evidence_json, &input.evidence_json);
    input
}

fn merge_text_set_json_values(existing: &Value, incoming: &Value) -> Value {
    let mut values = existing
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    values.extend(
        incoming
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    );
    json!(values.into_iter().collect::<Vec<_>>())
}
