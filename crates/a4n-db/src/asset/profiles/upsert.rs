use std::collections::BTreeSet;

use a4n_core::models::asset::object_profile;
use a4n_core::profile_ownership::{PROFILE_OWNERSHIP_AI_SYNC, PROFILE_OWNERSHIP_USER};
use a4n_core::profile_text::merge_profile_evidence_json;
use sea_orm::{ActiveModelTrait, ConnectionTrait};

use crate::asset::{AssetDbError, ObjectProfileRecordInput, ObjectProfileUpsertRecordResult};

use super::read::object_profile_by_normalized_name;
use super::record::{apply_object_profile_input, insert_object_profile_record};

pub(in crate::asset) async fn upsert_object_profiles_in<C>(
    db: &C,
    project_id: i64,
    inputs: Vec<ObjectProfileRecordInput>,
) -> Result<ObjectProfileUpsertRecordResult, AssetDbError>
where
    C: ConnectionTrait,
{
    let mut result = ObjectProfileUpsertRecordResult::default();
    let mut seen = BTreeSet::new();
    for input in inputs {
        if !seen.insert(input.normalized_name.clone()) {
            return Err(AssetDbError::InvalidArgument(format!(
                "物体发现结果存在重复物体: {}",
                input.name
            )));
        }
        match object_profile_by_normalized_name(db, project_id, &input.normalized_name).await? {
            Some(model) => result
                .updated
                .push(merge_object_profile_record(db, model, input).await?),
            None => result.created.push(
                insert_object_profile_record(db, project_id, input, PROFILE_OWNERSHIP_AI_SYNC)
                    .await?,
            ),
        }
    }
    Ok(result)
}

async fn merge_object_profile_record<C>(
    db: &C,
    existing: object_profile::Model,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    let input = merge_object_profile_record_input(&existing, input);
    if object_profile_matches_input(&existing, &input) {
        return Ok(existing);
    }
    let mut active: object_profile::ActiveModel = existing.into();
    apply_object_profile_input(&mut active, input);
    active.update(db).await.map_err(AssetDbError::Database)
}

fn merge_object_profile_record_input(
    existing: &object_profile::Model,
    mut input: ObjectProfileRecordInput,
) -> ObjectProfileRecordInput {
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
    if existing.ownership_kind != PROFILE_OWNERSHIP_AI_SYNC {
        input = merge_user_object_profile_fields(existing, input);
    }
    input.first_seen_chapter = first_seen_chapter;
    input.last_seen_chapter = last_seen_chapter;
    input.evidence_json = evidence_json;
    input
}

fn merge_user_object_profile_fields(
    existing: &object_profile::Model,
    incoming: ObjectProfileRecordInput,
) -> ObjectProfileRecordInput {
    let mut merged = object_profile_record_input(existing);
    if existing.aliases_json.as_array().is_some_and(Vec::is_empty) {
        merged.aliases_json = incoming.aliases_json;
    }
    fill_blank(&mut merged.object_type, incoming.object_type);
    fill_blank(&mut merged.importance, incoming.importance);
    fill_blank(&mut merged.visual_description, incoming.visual_description);
    fill_blank(&mut merged.material_and_state, incoming.material_and_state);
    fill_blank(&mut merged.story_usage, incoming.story_usage);
    fill_blank(&mut merged.visual_usage, incoming.visual_usage);
    fill_blank(&mut merged.generation_usage, incoming.generation_usage);
    if merged
        .location_context
        .as_deref()
        .map(str::trim)
        .map(str::is_empty)
        .unwrap_or(true)
    {
        merged.location_context = incoming.location_context;
    }
    fill_blank(&mut merged.image_prompt_seed, incoming.image_prompt_seed);
    merged
}

fn fill_blank(current: &mut String, incoming: String) {
    if current.trim().is_empty() {
        *current = incoming;
    }
}

fn object_profile_record_input(existing: &object_profile::Model) -> ObjectProfileRecordInput {
    ObjectProfileRecordInput {
        name: existing.name.clone(),
        normalized_name: existing.normalized_name.clone(),
        aliases_json: existing.aliases_json.clone(),
        object_type: existing.object_type.clone(),
        importance: existing.importance.clone(),
        visual_description: existing.visual_description.clone(),
        material_and_state: existing.material_and_state.clone(),
        story_usage: existing.story_usage.clone(),
        visual_usage: existing.visual_usage.clone(),
        generation_usage: existing.generation_usage.clone(),
        location_context: existing.location_context.clone(),
        first_seen_chapter: existing.first_seen_chapter,
        last_seen_chapter: existing.last_seen_chapter,
        evidence_json: existing.evidence_json.clone(),
        image_prompt_seed: existing.image_prompt_seed.clone(),
    }
}

fn object_profile_matches_input(
    existing: &object_profile::Model,
    input: &ObjectProfileRecordInput,
) -> bool {
    existing.name == input.name
        && existing.normalized_name == input.normalized_name
        && existing.aliases_json == input.aliases_json
        && existing.object_type == input.object_type
        && existing.importance == input.importance
        && existing.visual_description == input.visual_description
        && existing.material_and_state == input.material_and_state
        && existing.story_usage == input.story_usage
        && existing.visual_usage == input.visual_usage
        && existing.generation_usage == input.generation_usage
        && existing.location_context == input.location_context
        && existing.first_seen_chapter == input.first_seen_chapter
        && existing.last_seen_chapter == input.last_seen_chapter
        && existing.evidence_json == input.evidence_json
        && existing.image_prompt_seed == input.image_prompt_seed
}

#[cfg(test)]
mod tests {
    use a4n_core::profile_ownership::{PROFILE_OWNERSHIP_AI_SYNC, PROFILE_OWNERSHIP_USER};
    use chrono::NaiveDateTime;
    use serde_json::json;

    use super::*;

    fn profile(ownership_kind: &str) -> object_profile::Model {
        object_profile::Model {
            id: 1,
            project_id: 2,
            ownership_kind: ownership_kind.to_string(),
            name: "用户怀表".to_string(),
            normalized_name: "用户怀表".to_string(),
            aliases_json: json!(["旧称"]),
            object_type: "token".to_string(),
            importance: "primary".to_string(),
            visual_description: "用户外观".to_string(),
            material_and_state: "用户状态".to_string(),
            story_usage: "用户用途".to_string(),
            visual_usage: "用户视觉用途".to_string(),
            generation_usage: "用户生成用途".to_string(),
            location_context: Some("用户位置".to_string()),
            first_seen_chapter: 2,
            last_seen_chapter: 4,
            evidence_json: json!(["旧证据"]),
            image_prompt_seed: "用户提示词".to_string(),
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
        }
    }

    fn discovery_input() -> ObjectProfileRecordInput {
        ObjectProfileRecordInput {
            name: "AI 怀表".to_string(),
            normalized_name: "用户怀表".to_string(),
            aliases_json: json!(["AI 别名"]),
            object_type: "prop".to_string(),
            importance: "minor".to_string(),
            visual_description: "AI 外观".to_string(),
            material_and_state: "AI 状态".to_string(),
            story_usage: "AI 用途".to_string(),
            visual_usage: "AI 视觉用途".to_string(),
            generation_usage: "AI 生成用途".to_string(),
            location_context: Some("AI 位置".to_string()),
            first_seen_chapter: 1,
            last_seen_chapter: 8,
            evidence_json: json!(["新证据"]),
            image_prompt_seed: "AI 提示词".to_string(),
        }
    }

    #[test]
    fn profile_merge_preserves_user_fields_fills_blanks_and_extends_facts() {
        let mut existing = profile(PROFILE_OWNERSHIP_USER);
        existing.material_and_state.clear();
        let merged = merge_object_profile_record_input(&existing, discovery_input());

        assert_eq!(merged.name, existing.name);
        assert_eq!(merged.visual_description, existing.visual_description);
        assert_eq!(merged.material_and_state, "AI 状态");
        assert_eq!(merged.first_seen_chapter, 1);
        assert_eq!(merged.last_seen_chapter, 8);
        assert_eq!(merged.evidence_json, json!(["新证据", "旧证据"]));
    }

    #[test]
    fn profile_merge_can_refresh_ai_owned_fields() {
        let existing = profile(PROFILE_OWNERSHIP_AI_SYNC);
        let merged = merge_object_profile_record_input(&existing, discovery_input());

        assert_eq!(merged.name, "AI 怀表");
        assert_eq!(merged.visual_description, "AI 外观");
        assert_eq!(merged.first_seen_chapter, 1);
        assert_eq!(merged.last_seen_chapter, 8);
    }

    #[test]
    fn profile_merge_replaces_unconfirmed_manual_chapter_placeholder() {
        let mut existing = profile(PROFILE_OWNERSHIP_USER);
        existing.first_seen_chapter = 1;
        existing.last_seen_chapter = 1;
        existing.evidence_json = json!([]);
        let mut incoming = discovery_input();
        incoming.first_seen_chapter = 5;
        incoming.last_seen_chapter = 5;

        let merged = merge_object_profile_record_input(&existing, incoming);

        assert_eq!(merged.first_seen_chapter, 5);
        assert_eq!(merged.last_seen_chapter, 5);
    }
}
