use super::super::evidence::resolve_scene_text_anchor_texts;
use super::super::types::SceneProfileSyncRequestContext;
use super::validation::{validate_character_refs, validate_object_refs};
use crate::scene_profile::SceneOccurrenceDraft;
use a4n_core::json_contract::{
    ensure_object, optional_string, positive_i64_array, reject_unknown_fields, required_array,
    required_object_value, required_string, string_array_value,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn parse_scene_occurrences(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<SceneOccurrenceDraft>, String> {
    let occurrences = required_array(root, "occurrences", "root")?;
    let mut refs = BTreeSet::new();
    let mut drafts = Vec::with_capacity(occurrences.len());
    for (index, occurrence) in occurrences.iter().enumerate() {
        let path = format!("occurrences[{index}]");
        ensure_object(occurrence, &path)?;
        reject_unknown_fields(
            occurrence,
            &path,
            &[
                "occurrence_ref",
                "scene_ref",
                "timeline_position",
                "summary",
                "mood",
                "weather_or_lighting",
                "source_anchor_ids",
                "characters",
                "objects",
                "key_events",
                "entry_state",
                "exit_state",
            ],
        )?;
        let occurrence_ref = required_string(occurrence, "occurrence_ref", &path)?;
        if !refs.insert(occurrence_ref.clone()) {
            return Err(format!(
                "{path}.occurrence_ref 与前序出场重复: {occurrence_ref}"
            ));
        }
        let source_anchor_ids = positive_i64_array(occurrence, "source_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.source_anchor_ids"),
            &source_anchor_ids,
            context,
        )?;
        drafts.push(SceneOccurrenceDraft {
            occurrence_ref,
            scene_ref: required_string(occurrence, "scene_ref", &path)?,
            occurrence_order: i32::try_from(index + 1)
                .map_err(|_| format!("{path}.occurrence_order 超出 i32 范围"))?,
            timeline_position: optional_string(occurrence, "timeline_position", &path)?,
            summary: required_string(occurrence, "summary", &path)?,
            mood: optional_string(occurrence, "mood", &path)?,
            weather_or_lighting: optional_string(occurrence, "weather_or_lighting", &path)?,
            source_anchor_ids,
            evidence,
            characters: validate_character_refs(
                occurrence,
                "characters",
                &path,
                &context.character_ids,
            )?,
            objects: validate_object_refs(occurrence, "objects", &path, &context.object_ids)?,
            key_events: string_array_value(occurrence, "key_events", &path, false)?,
            entry_state: required_object_value(occurrence, "entry_state", &path)?,
            exit_state: required_object_value(occurrence, "exit_state", &path)?,
        });
    }
    Ok(drafts)
}
