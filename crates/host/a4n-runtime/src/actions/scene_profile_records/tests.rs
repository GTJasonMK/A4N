use serde_json::json;

#[test]
fn scene_profile_summary_shape_keeps_occurrence_and_image_counts() {
    let value = json!({
        "id": 1,
        "project_id": 2,
        "name": "长安东市",
        "normalized_name": "长安东市",
        "aliases_json": [],
        "scene_type": "market",
        "importance": "high",
        "parent_scene_id": null,
        "base_description": "繁华市井",
        "visual_description": "灯火与摊贩",
        "spatial_layout": "街巷纵横",
        "atmosphere": "喧闹",
        "entry_points_json": [],
        "rules_json": [],
        "current_state_json": {},
        "first_seen_chapter": 1,
        "last_seen_chapter": 3,
        "evidence_json": [],
        "image_prompt_seed": "ancient market",
        "occurrence_count": 5,
        "generated_image_count": 2,
        "created_at": "2026-01-01 00:00:00",
        "updated_at": "2026-01-01 00:00:00"
    });
    assert_eq!(value["occurrence_count"], json!(5));
    assert_eq!(value["generated_image_count"], json!(2));
}
