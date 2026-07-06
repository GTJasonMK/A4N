use super::super::*;
use serde_json::json;

#[test]
fn snapshot_changes_use_persisted_attribute_events() {
    let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 5, 27)
        .and_then(|date| date.and_hms_opt(9, 30, 0))
        .expect("valid timestamp");
    let events = vec![
        attribute_event::Model {
            id: 11,
            profile_id: 7,
            chapter_number: 2,
            timestamp,
            category: "implicit".to_string(),
            key: "谨慎".to_string(),
            operation: "modify".to_string(),
            change_description: "行为复审后调整谨慎定义".to_string(),
            old_value: json!("遇事先观察"),
            new_value: json!("观察后主动行动"),
            cause: "隐性属性复审".to_string(),
            evidence: "他观察片刻后主动上前。".to_string(),
        },
        attribute_event::Model {
            id: 12,
            profile_id: 7,
            chapter_number: 3,
            timestamp,
            category: "explicit".to_string(),
            key: "位置".to_string(),
            operation: "modify".to_string(),
            change_description: "抵达渡口".to_string(),
            old_value: json!("落霞镇"),
            new_value: json!("渡口"),
            cause: "赶路抵达".to_string(),
            evidence: "渡口灯火已近。".to_string(),
        },
    ];

    let changes = attribute_events_at_chapter(&events, 2);

    assert_eq!(changes[0]["id"], json!(11));
    assert_eq!(changes[0]["key"], json!("谨慎"));
    assert_eq!(
        changes[0]["change_description"],
        json!("行为复审后调整谨慎定义")
    );
    assert_eq!(changes[0]["old_value"], json!("遇事先观察"));
    assert_eq!(changes.as_array().expect("array").len(), 1);
}
