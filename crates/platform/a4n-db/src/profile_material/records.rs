use a4n_db_models::models::{
    asset::{object_profile, profile_primary_material},
    scene::scene_profile,
};
use a4n_domain::profile_material::{
    ProfileMaterialObjectRecord, ProfileMaterialSceneRecord, ProfilePrimaryMaterialRecord,
};

pub fn scene_profile_source_record(profile: &scene_profile::Model) -> ProfileMaterialSceneRecord {
    ProfileMaterialSceneRecord {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        scene_type: profile.scene_type.clone(),
        importance: profile.importance.clone(),
        base_description: profile.base_description.clone(),
        visual_description: profile.visual_description.clone(),
        spatial_layout: profile.spatial_layout.clone(),
        atmosphere: profile.atmosphere.clone(),
        entry_points_json: profile.entry_points_json.clone(),
        rules_json: profile.rules_json.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        image_prompt_seed: profile.image_prompt_seed.clone(),
        updated_at: profile.updated_at,
    }
}

pub fn object_profile_source_record(
    profile: &object_profile::Model,
) -> ProfileMaterialObjectRecord {
    ProfileMaterialObjectRecord {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        object_type: profile.object_type.clone(),
        importance: profile.importance.clone(),
        visual_description: profile.visual_description.clone(),
        material_and_state: profile.material_and_state.clone(),
        story_usage: profile.story_usage.clone(),
        visual_usage: profile.visual_usage.clone(),
        generation_usage: profile.generation_usage.clone(),
        location_context: profile.location_context.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        image_prompt_seed: profile.image_prompt_seed.clone(),
        updated_at: profile.updated_at,
    }
}

pub(super) fn primary_material_record(
    material: profile_primary_material::Model,
) -> ProfilePrimaryMaterialRecord {
    ProfilePrimaryMaterialRecord {
        profile_kind: material.profile_kind,
        profile_id: material.profile_id,
        primary_image_id: material.primary_image_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn source_records_preserve_material_fields() {
        let updated_at = NaiveDate::from_ymd_opt(2026, 9, 30)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let scene = scene_profile::Model {
            id: 11,
            project_id: 12,
            ownership_kind: "user".into(),
            name: "旧书房".into(),
            normalized_name: "旧书房".into(),
            aliases_json: json!([]),
            scene_type: "room".into(),
            importance: "primary".into(),
            parent_scene_id: None,
            base_description: "旧屋".into(),
            visual_description: "落满灰尘".into(),
            spatial_layout: "书架环绕".into(),
            atmosphere: "安静".into(),
            entry_points_json: json!(["木门"]),
            rules_json: json!(["夜间锁门"]),
            current_state_json: json!({}),
            first_seen_chapter: 2,
            last_seen_chapter: 5,
            evidence_json: json!([]),
            image_prompt_seed: "暖色台灯".into(),
            created_at: updated_at,
            updated_at,
        };
        let scene_record = scene_profile_source_record(&scene);
        assert_eq!(
            (scene_record.id, scene_record.project_id),
            (scene.id, scene.project_id)
        );
        assert_eq!(scene_record.name, scene.name);
        assert_eq!(scene_record.scene_type, scene.scene_type);
        assert_eq!(scene_record.importance, scene.importance);
        assert_eq!(scene_record.base_description, scene.base_description);
        assert_eq!(scene_record.visual_description, scene.visual_description);
        assert_eq!(scene_record.spatial_layout, scene.spatial_layout);
        assert_eq!(scene_record.atmosphere, scene.atmosphere);
        assert_eq!(scene_record.entry_points_json, scene.entry_points_json);
        assert_eq!(scene_record.rules_json, scene.rules_json);
        assert_eq!(
            (
                scene_record.first_seen_chapter,
                scene_record.last_seen_chapter
            ),
            (scene.first_seen_chapter, scene.last_seen_chapter)
        );
        assert_eq!(scene_record.image_prompt_seed, scene.image_prompt_seed);
        assert_eq!(scene_record.updated_at, scene.updated_at);

        let object = object_profile::Model {
            id: 21,
            project_id: 22,
            ownership_kind: "user".into(),
            name: "青铜钥匙".into(),
            normalized_name: "青铜钥匙".into(),
            aliases_json: json!([]),
            object_type: "prop".into(),
            importance: "secondary".into(),
            visual_description: "有裂纹".into(),
            material_and_state: "氧化青铜".into(),
            story_usage: "开启旧门".into(),
            visual_usage: "开门特写".into(),
            generation_usage: "保持裂纹".into(),
            location_context: Some("旧书房".into()),
            first_seen_chapter: 3,
            last_seen_chapter: 8,
            evidence_json: json!([]),
            image_prompt_seed: "铜绿".into(),
            created_at: updated_at,
            updated_at,
        };
        let object_record = object_profile_source_record(&object);
        assert_eq!(
            (object_record.id, object_record.project_id),
            (object.id, object.project_id)
        );
        assert_eq!(object_record.name, object.name);
        assert_eq!(object_record.object_type, object.object_type);
        assert_eq!(object_record.importance, object.importance);
        assert_eq!(object_record.visual_description, object.visual_description);
        assert_eq!(object_record.material_and_state, object.material_and_state);
        assert_eq!(object_record.story_usage, object.story_usage);
        assert_eq!(object_record.visual_usage, object.visual_usage);
        assert_eq!(object_record.generation_usage, object.generation_usage);
        assert_eq!(object_record.location_context, object.location_context);
        assert_eq!(
            (
                object_record.first_seen_chapter,
                object_record.last_seen_chapter
            ),
            (object.first_seen_chapter, object.last_seen_chapter)
        );
        assert_eq!(object_record.image_prompt_seed, object.image_prompt_seed);
        assert_eq!(object_record.updated_at, object.updated_at);
    }

    #[test]
    fn primary_record_keeps_binding_identity() {
        let updated_at = NaiveDate::from_ymd_opt(2026, 9, 30)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let record = primary_material_record(profile_primary_material::Model {
            id: 31,
            project_id: 32,
            profile_kind: "scene".into(),
            profile_id: 33,
            primary_image_id: 34,
            created_at: updated_at,
            updated_at,
        });

        assert_eq!(record.profile_kind, "scene");
        assert_eq!(record.profile_id, 33);
        assert_eq!(record.primary_image_id, 34);
    }
}
