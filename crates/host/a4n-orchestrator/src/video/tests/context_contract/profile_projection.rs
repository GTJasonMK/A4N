#[test]
fn video_context_character_names_collects_analysis_names() {
    let names = video_context_character_names(Some(&json!({
        "metadata": {
            "characters": ["林岚", {"name": "周澈", "aliases": ["周先生"]}]
        },
        "character_states": {
            "沈砚": {"status": "受伤"}
        },
        "key_events": [{
            "involved_characters": ["林岚", "陈栖"]
        }]
    })));

    assert!(names.contains("林岚"));
    assert!(names.contains("周澈"));
    assert!(names.contains("周先生"));
    assert!(names.contains("沈砚"));
    assert!(names.contains("陈栖"));
}

#[test]
fn video_profile_context_is_visual_compact_without_database_metadata() {
    let profile = a4n_db::video::VideoPromptProtagonistProfileRecord {
        name: "林岚".to_string(),
        gender: Some("女".to_string()),
        age: Some("二十岁".to_string()),
        base_attributes: json!({}),
        current_state: json!({
            "explicit_attributes": {
                "appearance": "短发黑衣",
                "role": "调查员"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        appearance: Some("短发黑衣".to_string()),
        background: Some("旧城调查员".to_string()),
        personality: Some("冷静".to_string()),
    };

    let value = video_profile_context(&profile);

    assert_eq!(value["appearance"], json!("短发黑衣"));
    assert_eq!(
        value["current_explicit_attributes"]["role"],
        json!("调查员")
    );
    assert!(value.get("id").is_none());
    assert!(value.get("project_id").is_none());
    assert!(value.get("created_at").is_none());
    assert!(value.get("updated_at").is_none());
    assert!(value.get("base_attributes").is_none());
}
