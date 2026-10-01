use a4n_db_models::models::asset::object_profile;
use a4n_domain::object_profile::ObjectProfileRecord;
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_USER;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::asset::AssetDbError;
use a4n_domain::object_profile::ObjectProfileRecordInput;

pub(crate) fn object_profile_record(model: object_profile::Model) -> ObjectProfileRecord {
    ObjectProfileRecord {
        id: model.id,
        project_id: model.project_id,
        ownership_kind: model.ownership_kind,
        name: model.name,
        normalized_name: model.normalized_name,
        aliases_json: model.aliases_json,
        object_type: model.object_type,
        importance: model.importance,
        visual_description: model.visual_description,
        material_and_state: model.material_and_state,
        story_usage: model.story_usage,
        visual_usage: model.visual_usage,
        generation_usage: model.generation_usage,
        location_context: model.location_context,
        first_seen_chapter: model.first_seen_chapter,
        last_seen_chapter: model.last_seen_chapter,
        evidence_json: model.evidence_json,
        image_prompt_seed: model.image_prompt_seed,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn object_profile_model_from_record(
    record: &ObjectProfileRecord,
) -> object_profile::Model {
    object_profile::Model {
        id: record.id,
        project_id: record.project_id,
        ownership_kind: record.ownership_kind.clone(),
        name: record.name.clone(),
        normalized_name: record.normalized_name.clone(),
        aliases_json: record.aliases_json.clone(),
        object_type: record.object_type.clone(),
        importance: record.importance.clone(),
        visual_description: record.visual_description.clone(),
        material_and_state: record.material_and_state.clone(),
        story_usage: record.story_usage.clone(),
        visual_usage: record.visual_usage.clone(),
        generation_usage: record.generation_usage.clone(),
        location_context: record.location_context.clone(),
        first_seen_chapter: record.first_seen_chapter,
        last_seen_chapter: record.last_seen_chapter,
        evidence_json: record.evidence_json.clone(),
        image_prompt_seed: record.image_prompt_seed.clone(),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub(super) async fn insert_object_profile_record<C>(
    db: &C,
    project_id: i64,
    input: ObjectProfileRecordInput,
    ownership_kind: &str,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    object_profile_active_model(project_id, input, ownership_kind, now)
        .insert(db)
        .await
        .map_err(AssetDbError::Database)
}

pub(super) fn object_profile_active_model(
    project_id: i64,
    input: ObjectProfileRecordInput,
    ownership_kind: &str,
    now: chrono::NaiveDateTime,
) -> object_profile::ActiveModel {
    object_profile::ActiveModel {
        project_id: Set(project_id),
        ownership_kind: Set(ownership_kind.to_string()),
        name: Set(input.name),
        normalized_name: Set(input.normalized_name),
        aliases_json: Set(input.aliases_json),
        object_type: Set(input.object_type),
        importance: Set(input.importance),
        visual_description: Set(input.visual_description),
        material_and_state: Set(input.material_and_state),
        story_usage: Set(input.story_usage),
        visual_usage: Set(input.visual_usage),
        generation_usage: Set(input.generation_usage),
        location_context: Set(input.location_context),
        first_seen_chapter: Set(input.first_seen_chapter),
        last_seen_chapter: Set(input.last_seen_chapter),
        evidence_json: Set(input.evidence_json),
        image_prompt_seed: Set(input.image_prompt_seed),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

pub(super) async fn replace_object_profile_record<C>(
    db: &C,
    existing: object_profile::Model,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    let mut active: object_profile::ActiveModel = existing.into();
    apply_object_profile_input(&mut active, input);
    active.ownership_kind = Set(PROFILE_OWNERSHIP_USER.to_string());
    active.update(db).await.map_err(AssetDbError::Database)
}

pub(super) fn apply_object_profile_input(
    active: &mut object_profile::ActiveModel,
    input: ObjectProfileRecordInput,
) {
    active.name = Set(input.name);
    active.normalized_name = Set(input.normalized_name);
    active.aliases_json = Set(input.aliases_json);
    active.object_type = Set(input.object_type);
    active.importance = Set(input.importance);
    active.visual_description = Set(input.visual_description);
    active.material_and_state = Set(input.material_and_state);
    active.story_usage = Set(input.story_usage);
    active.visual_usage = Set(input.visual_usage);
    active.generation_usage = Set(input.generation_usage);
    active.location_context = Set(input.location_context);
    active.first_seen_chapter = Set(input.first_seen_chapter);
    active.last_seen_chapter = Set(input.last_seen_chapter);
    active.evidence_json = Set(input.evidence_json);
    active.image_prompt_seed = Set(input.image_prompt_seed);
    active.updated_at = Set(Utc::now().naive_utc());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn object_profile_record_preserves_storage_and_package_shape() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let model = object_profile::Model {
            id: 1,
            project_id: 2,
            ownership_kind: "user".to_string(),
            name: "旧怀表".to_string(),
            normalized_name: "旧怀表".to_string(),
            aliases_json: json!(["祖父的怀表"]),
            object_type: "prop".to_string(),
            importance: "primary".to_string(),
            visual_description: "磨损的黄铜表壳".to_string(),
            material_and_state: "黄铜，停摆".to_string(),
            story_usage: "遗物".to_string(),
            visual_usage: "近景道具".to_string(),
            generation_usage: "保留划痕".to_string(),
            location_context: Some("旧宅书房".to_string()),
            first_seen_chapter: 1,
            last_seen_chapter: 3,
            evidence_json: json!(["正文证据"]),
            image_prompt_seed: "vintage pocket watch".to_string(),
            created_at,
            updated_at: created_at,
        };

        let record = object_profile_record(model.clone());

        assert_eq!(object_profile_model_from_record(&record), model);
        assert_eq!(
            serde_json::to_value(record).unwrap(),
            serde_json::to_value(model).unwrap()
        );
    }
}
