use super::merge::merge_missing_seed_attributes;
use a4n_domain::json_contract::is_empty_json_value;
use serde_json::{Value, json};

#[test]
fn blueprint_seed_preserves_deleted_attributes_after_timeline_start() {
    use super::{contracts::BlueprintProfileSeed, persistence::merge_profile_seed_model};
    use a4n_db_models::models::character::protagonist_profile;

    let now = chrono::Utc::now().naive_utc();
    let base = json!({
        "explicit_attributes": {"appearance": "右臂有纹身"},
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let mut current = base.clone();
    a4n_domain::character::apply_attribute_to_state(
        &mut current,
        "explicit",
        "appearance",
        "delete",
        Value::Null,
    )
    .unwrap();
    let profile = protagonist_profile::Model {
        id: 1,
        project_id: 1,
        name: "林岚".into(),
        gender: None,
        age: None,
        base_attributes: base.clone(),
        current_state: current,
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 3,
        created_at: now,
        updated_at: now,
    };
    let seed = BlueprintProfileSeed {
        base_attributes: base,
        character_arc: None,
    };
    assert!(merge_profile_seed_model(profile.clone(), &seed, now, false).is_none());
    let mut manual_timeline = profile;
    manual_timeline.last_synced_chapter = 0;
    assert!(merge_profile_seed_model(manual_timeline, &seed, now, true).is_none());
}

#[test]
fn merge_missing_seed_attributes_only_fills_empty_values() {
    let mut target = json!({
        "explicit_attributes": {
            "gender": "女",
            "age": ""
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let seed = json!({
        "explicit_attributes": {
            "gender": "男",
            "age": "20",
            "appearance": "黑发"
        },
        "implicit_attributes": {
            "personality": "冷静"
        },
        "social_attributes": {}
    });

    assert!(merge_missing_seed_attributes(&mut target, &seed));
    assert_eq!(target["explicit_attributes"]["gender"], json!("女"));
    assert_eq!(target["explicit_attributes"]["age"], json!("20"));
    assert_eq!(target["explicit_attributes"]["appearance"], json!("黑发"));
    assert_eq!(target["implicit_attributes"]["personality"], json!("冷静"));
}

#[test]
fn seed_value_empty_contract_preserves_false_and_zero() {
    assert!(is_empty_json_value(&Value::Null));
    assert!(is_empty_json_value(&json!("  ")));
    assert!(is_empty_json_value(&json!([])));
    assert!(is_empty_json_value(&json!({})));
    assert!(!is_empty_json_value(&json!(false)));
    assert!(!is_empty_json_value(&json!(0)));
}
