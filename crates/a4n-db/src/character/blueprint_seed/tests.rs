use super::merge::merge_missing_seed_attributes;
use a4n_domain::json_contract::is_empty_json_value;
use serde_json::{Value, json};

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
