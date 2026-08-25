use a4n_core::models::asset::object_profile;
use a4n_core::profile_ownership::PROFILE_OWNERSHIP_USER;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::asset::{AssetDbError, ObjectProfileRecordInput};

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
