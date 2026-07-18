use serde_json::Value;

use super::super::CharacterSyncStage;

pub(super) fn required_character_sync_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, String> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.{field} 必须是字符串"))
}

pub(super) fn optional_character_sync_checkpoint_string(
    checkpoint_json: &Value,
    field: &str,
) -> Result<Option<String>, String> {
    match checkpoint_json.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| format!("{field} 必须是字符串或 null")),
    }
}

pub(in crate::actions::character_ai::sync_profile) fn stage_temperature(
    stage: CharacterSyncStage,
) -> f32 {
    match stage {
        CharacterSyncStage::EvidenceEvents => 0.2,
        CharacterSyncStage::AttributeChanges => 0.25,
        CharacterSyncStage::ChapterStates => 0.25,
        CharacterSyncStage::BehaviorRecords => 0.25,
        CharacterSyncStage::DeletionCandidates => 0.15,
    }
}
