use a4n_db_models::models::scene::scene_profile;
use a4n_domain::scene_profile::SceneProfileRecordInput;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::json;

use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};

pub async fn create_scene_profile_record<C>(
    db: &C,
    project_id: i64,
    input: SceneProfileRecordInput,
    ownership_kind: &str,
) -> SceneProfileDbResult<scene_profile::Model>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    scene_profile::ActiveModel {
        project_id: Set(project_id),
        ownership_kind: Set(ownership_kind.to_string()),
        name: Set(input.name),
        normalized_name: Set(input.normalized_name),
        aliases_json: Set(input.aliases_json),
        scene_type: Set(input.scene_type),
        importance: Set(input.importance),
        parent_scene_id: Set(input.parent_scene_id),
        base_description: Set(input.base_description),
        visual_description: Set(input.visual_description),
        spatial_layout: Set(input.spatial_layout),
        atmosphere: Set(input.atmosphere),
        entry_points_json: Set(input.entry_points_json),
        rules_json: Set(input.rules_json),
        current_state_json: Set(json!({})),
        first_seen_chapter: Set(input.first_seen_chapter),
        last_seen_chapter: Set(input.last_seen_chapter),
        evidence_json: Set(input.evidence_json),
        image_prompt_seed: Set(input.image_prompt_seed),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(SceneProfileDbError::Database)
}

pub async fn replace_scene_profile_record<C>(
    db: &C,
    existing: scene_profile::Model,
    input: SceneProfileRecordInput,
    ownership_kind: &str,
) -> SceneProfileDbResult<scene_profile::Model>
where
    C: ConnectionTrait,
{
    let mut active: scene_profile::ActiveModel = existing.into();
    active.ownership_kind = Set(ownership_kind.to_string());
    active.name = Set(input.name);
    active.normalized_name = Set(input.normalized_name);
    active.aliases_json = Set(input.aliases_json);
    active.scene_type = Set(input.scene_type);
    active.importance = Set(input.importance);
    active.parent_scene_id = Set(input.parent_scene_id);
    active.base_description = Set(input.base_description);
    active.visual_description = Set(input.visual_description);
    active.spatial_layout = Set(input.spatial_layout);
    active.atmosphere = Set(input.atmosphere);
    active.entry_points_json = Set(input.entry_points_json);
    active.rules_json = Set(input.rules_json);
    active.first_seen_chapter = Set(input.first_seen_chapter);
    active.last_seen_chapter = Set(input.last_seen_chapter);
    active.evidence_json = Set(input.evidence_json);
    active.image_prompt_seed = Set(input.image_prompt_seed);
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(SceneProfileDbError::Database)
}
