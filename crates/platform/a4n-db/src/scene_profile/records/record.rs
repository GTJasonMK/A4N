use a4n_db_models::models::scene::scene_profile;
use a4n_domain::scene_profile::{SceneProfileRecord, SceneProfileRecordInput};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::json;

use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};

pub(crate) fn scene_profile_record(model: scene_profile::Model) -> SceneProfileRecord {
    SceneProfileRecord {
        id: model.id,
        project_id: model.project_id,
        ownership_kind: model.ownership_kind,
        name: model.name,
        normalized_name: model.normalized_name,
        aliases_json: model.aliases_json,
        scene_type: model.scene_type,
        importance: model.importance,
        parent_scene_id: model.parent_scene_id,
        base_description: model.base_description,
        visual_description: model.visual_description,
        spatial_layout: model.spatial_layout,
        atmosphere: model.atmosphere,
        entry_points_json: model.entry_points_json,
        rules_json: model.rules_json,
        current_state_json: model.current_state_json,
        first_seen_chapter: model.first_seen_chapter,
        last_seen_chapter: model.last_seen_chapter,
        evidence_json: model.evidence_json,
        image_prompt_seed: model.image_prompt_seed,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn scene_profile_model_from_record(record: &SceneProfileRecord) -> scene_profile::Model {
    scene_profile::Model {
        id: record.id,
        project_id: record.project_id,
        ownership_kind: record.ownership_kind.clone(),
        name: record.name.clone(),
        normalized_name: record.normalized_name.clone(),
        aliases_json: record.aliases_json.clone(),
        scene_type: record.scene_type.clone(),
        importance: record.importance.clone(),
        parent_scene_id: record.parent_scene_id,
        base_description: record.base_description.clone(),
        visual_description: record.visual_description.clone(),
        spatial_layout: record.spatial_layout.clone(),
        atmosphere: record.atmosphere.clone(),
        entry_points_json: record.entry_points_json.clone(),
        rules_json: record.rules_json.clone(),
        current_state_json: record.current_state_json.clone(),
        first_seen_chapter: record.first_seen_chapter,
        last_seen_chapter: record.last_seen_chapter,
        evidence_json: record.evidence_json.clone(),
        image_prompt_seed: record.image_prompt_seed.clone(),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub(crate) async fn create_scene_profile_model<C>(
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

pub async fn create_scene_profile_record<C>(
    db: &C,
    project_id: i64,
    input: SceneProfileRecordInput,
    ownership_kind: &str,
) -> SceneProfileDbResult<SceneProfileRecord>
where
    C: ConnectionTrait,
{
    create_scene_profile_model(db, project_id, input, ownership_kind)
        .await
        .map(scene_profile_record)
}

pub(crate) async fn replace_scene_profile_record<C>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn scene_profile_record_preserves_storage_and_package_shape() {
        let now = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let model = scene_profile::Model {
            id: 1,
            project_id: 2,
            ownership_kind: "user".to_string(),
            name: "旧书房".to_string(),
            normalized_name: "旧书房".to_string(),
            aliases_json: json!(["藏书间"]),
            scene_type: "room".to_string(),
            importance: "primary".to_string(),
            parent_scene_id: None,
            base_description: "木屋中的书房".to_string(),
            visual_description: "尘埃落在木桌".to_string(),
            spatial_layout: "书架环绕房间".to_string(),
            atmosphere: "安静".to_string(),
            entry_points_json: json!(["南门"]),
            rules_json: json!(["夜间不点灯"]),
            current_state_json: json!({"weather_or_lighting": "昏暗"}),
            first_seen_chapter: 1,
            last_seen_chapter: 3,
            evidence_json: json!(["正文证据"]),
            image_prompt_seed: "old study".to_string(),
            created_at: now,
            updated_at: now,
        };

        let record = scene_profile_record(model.clone());

        assert_eq!(scene_profile_model_from_record(&record), model);
        assert_eq!(
            serde_json::to_value(record).unwrap(),
            serde_json::to_value(model).unwrap()
        );
    }
}
