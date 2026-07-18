use serde_json::json;

#[test]
fn object_profile_summary_shape_keeps_generated_image_count_field() {
    let value = json!({
        "id": 1,
        "project_id": 2,
        "name": "玉佩",
        "normalized_name": "玉佩",
        "aliases_json": [],
        "object_type": "prop",
        "importance": "high",
        "visual_description": "青白色玉佩",
        "material_and_state": "温润完整",
        "story_usage": "信物",
        "visual_usage": "锁定角色手中信物",
        "generation_usage": "约束玉佩外观",
        "location_context": null,
        "first_seen_chapter": 1,
        "last_seen_chapter": 3,
        "evidence_json": [],
        "image_prompt_seed": "jade pendant",
        "generated_image_count": 2,
        "created_at": "2026-01-01 00:00:00",
        "updated_at": "2026-01-01 00:00:00"
    });
    assert_eq!(value["generated_image_count"], json!(2));
}
