use super::projections::{attribute_counts, snapshot_data_summary};
use a4n_api_contract::character::ListSnapshotsInput;
use a4n_core::models::character::snapshot_reference;
use chrono::Utc;
use serde_json::json;

#[test]
fn attribute_counts_requires_canonical_category_objects() {
    let counts = attribute_counts(&json!({
        "explicit_attributes": {"appearance": "黑衣"},
        "implicit_attributes": {},
        "social_attributes": {"身份": "密探"}
    }))
    .expect("complete attribute categories should count");

    assert_eq!(counts.explicit, 1);
    assert_eq!(counts.implicit, 0);
    assert_eq!(counts.social, 1);

    let missing = attribute_counts(&json!({
        "explicit_attributes": {},
        "implicit_attributes": {}
    }))
    .expect_err("missing social attributes should fail");
    assert!(missing.contains("角色属性缺少分类 social_attributes"));

    let corrupt = attribute_counts(&json!({
        "explicit_attributes": [],
        "implicit_attributes": {},
        "social_attributes": {}
    }))
    .expect_err("corrupt explicit attributes should fail");
    assert!(corrupt.contains("角色属性分类 explicit_attributes 必须是对象"));
}

#[test]
fn snapshot_data_summary_requires_explicit_chapter_counters() {
    let now = Utc::now().naive_utc();
    let mut data = json!({
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        },
        "state_summary": "潜入府衙",
        "current_location": "府衙",
        "changes_in_chapter": 2,
        "behaviors_in_chapter": 3
    });
    let mut snapshot = snapshot_reference::Model {
        id: 10,
        profile_id: 7,
        chapter_number: 4,
        snapshot_hash: "abc123".to_string(),
        reference_count: 1,
        data: data.clone(),
        created_at: now,
    };

    let summary = snapshot_data_summary(&snapshot).expect("valid snapshot should summarize");
    assert_eq!(summary.changes_in_chapter, 2);
    assert_eq!(summary.behaviors_in_chapter, 3);

    data.as_object_mut()
        .expect("object")
        .remove("changes_in_chapter");
    snapshot.data = data;
    let err = snapshot_data_summary(&snapshot).expect_err("missing explicit counter should fail");
    assert!(err.contains("changes_in_chapter"));
}

#[test]
fn list_snapshots_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<ListSnapshotsInput>(json!({
        "profile_id": 1,
        "start_chapter": null,
        "unexpected": true
    }))
    .expect_err("unknown fields should be rejected");

    assert!(err.to_string().contains("unknown field"));
}
