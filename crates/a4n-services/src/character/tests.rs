use super::*;
use serde_json::json;

#[test]
fn apply_current_state_refreshes_profile_summary_fields() {
    let state = json!({
        "explicit_attributes": {
            "gender": "女",
            "age": "二十七",
            "appearance": "黑衣",
            "background": "前朝密探"
        },
        "implicit_attributes": {
            "personality": "谨慎"
        },
        "social_attributes": {}
    });
    let mut active: protagonist_profile::ActiveModel = Default::default();

    apply_profile_current_state(&mut active, state.clone());

    assert_eq!(active.gender, Set(Some("女".to_string())));
    assert_eq!(active.age, Set(Some("二十七".to_string())));
    assert_eq!(active.appearance, Set(Some("黑衣".to_string())));
    assert_eq!(active.background, Set(Some("前朝密探".to_string())));
    assert_eq!(active.personality, Set(Some("谨慎".to_string())));
    assert_eq!(active.current_state, Set(state));
}

#[test]
fn normalize_base_attributes_accepts_canonical_state() {
    let normalized = normalize_base_attributes(json!({
        "explicit_attributes": {
            "gender": "女",
            "appearance": "黑衣"
        },
        "implicit_attributes": {
            "personality": "谨慎"
        },
        "social_attributes": {}
    }))
    .expect("canonical initial state should pass");

    assert_eq!(normalized["explicit_attributes"]["gender"], json!("女"));
    assert_eq!(
        normalized["explicit_attributes"]["appearance"],
        json!("黑衣")
    );
    assert_eq!(
        normalized["implicit_attributes"]["personality"],
        json!("谨慎")
    );
    assert_eq!(normalized["social_attributes"], json!({}));
}

#[test]
fn normalize_base_attributes_rejects_flat_profile_form() {
    let err = normalize_base_attributes(json!({
        "gender": "女",
        "personality": "谨慎"
    }))
    .expect_err("flat create form should fail");

    assert!(err.to_string().contains("explicit_attributes"));
}

#[test]
fn normalize_base_attributes_rejects_partial_canonical_state() {
    let err = normalize_base_attributes(json!({
        "explicit_attributes": {}
    }))
    .expect_err("canonical initial state must include all categories");

    assert!(err.to_string().contains("implicit_attributes"));
}

#[test]
fn apply_attribute_to_state_rejects_corrupt_state_shape() {
    let mut state = json!([]);
    let err = apply_attribute_to_state(
        &mut state,
        "explicit",
        "appearance",
        "modify",
        json!("白衣"),
    )
    .expect_err("non-object state must fail");

    assert!(err.to_string().contains("角色当前状态必须是 JSON 对象"));
}

#[test]
fn apply_attribute_to_state_rejects_corrupt_category_shape() {
    let mut state = json!({
        "explicit_attributes": [],
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let err = apply_attribute_to_state(
        &mut state,
        "explicit",
        "appearance",
        "modify",
        json!("白衣"),
    )
    .expect_err("non-object category must fail");

    assert!(err.to_string().contains("explicit_attributes"));
}

#[test]
fn behavior_classification_accepts_strict_values() {
    validate_behavior_classification_results(&json!({
        "谨慎": "conform",
        "冲动": "non-conform"
    }))
    .expect("strict classifications should pass");

    validate_behavior_classification_results(&json!({}))
        .expect("empty classifications should pass");
}

#[test]
fn behavior_classification_rejects_invalid_values() {
    let err = validate_behavior_classification_results(&json!({
        "谨慎": true
    }))
    .expect_err("non-string classification must fail");
    assert!(err.to_string().contains("conform/non-conform"));

    let err = validate_behavior_classification_results(&json!({
        "谨慎": "unknown"
    }))
    .expect_err("unknown classification must fail");
    assert!(err.to_string().contains("conform/non-conform"));

    let err = validate_behavior_classification_results(&json!([]))
        .expect_err("array classification must fail");
    assert!(err.to_string().contains("JSON 对象"));
}

#[test]
fn deletion_mark_count_only_increments_on_next_chapter() {
    assert_eq!(next_deletion_mark_count(1, 3, 4), 2);
    assert_eq!(next_deletion_mark_count(4, 6, 7), 5);
    assert_eq!(next_deletion_mark_count(4, 3, 3), 1);
    assert_eq!(next_deletion_mark_count(4, 3, 5), 1);
    assert_eq!(next_deletion_mark_count(4, 3, 2), 1);
}

#[test]
fn deletion_mark_rewind_truncates_only_crossing_segments() {
    assert_eq!(retained_deletion_mark_count(3, 4), Some(2));
    assert_eq!(retained_deletion_mark_count(1, 4), Some(4));
    assert_eq!(retained_deletion_mark_count(5, 4), None);
    assert_eq!(retained_deletion_mark_count(0, 4), None);
    assert_eq!(retained_deletion_mark_count(1, 0), None);
}

#[test]
fn attribute_history_filter_rejects_invalid_ranges_and_values() {
    let err = validate_attribute_history_filter(&AttributeHistoryFilter {
        chapter_from: Some(5),
        chapter_to: Some(3),
        ..Default::default()
    })
    .expect_err("reversed chapter range should fail");
    assert!(err.to_string().contains("起始章节"));

    let err = validate_attribute_history_filter(&AttributeHistoryFilter {
        category: Some("other".to_string()),
        ..Default::default()
    })
    .expect_err("unknown category should fail");
    assert!(err.to_string().contains("explicit/implicit/social"));

    let err = validate_attribute_history_filter(&AttributeHistoryFilter {
        key: Some("  ".to_string()),
        ..Default::default()
    })
    .expect_err("empty key should fail");
    assert!(err.to_string().contains("属性 key"));

    let err = validate_attribute_history_filter(&AttributeHistoryFilter {
        operation: Some("replace".to_string()),
        ..Default::default()
    })
    .expect_err("unknown operation should fail");
    assert!(err.to_string().contains("add/modify/delete"));
}
