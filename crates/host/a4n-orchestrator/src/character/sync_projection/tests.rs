use serde_json::{Value, json};

use super::{pending_character_sync_attribute_events, project_character_attribute_changes};

#[test]
fn pending_events_keep_chapter_and_stage_order() {
    let changes = vec![
        json!({
            "chapter_number": 2,
            "category": "explicit",
            "key": "位置",
            "operation": "modify",
            "new_value": "落霞镇",
            "change_description": "抵达落霞镇",
            "event_cause": "赶路抵达",
            "evidence": "他终于看见落霞镇的牌坊"
        }),
        json!({
            "chapter_number": 3,
            "category": "social",
            "key": "少女",
            "operation": "add",
            "new_value": "被主角保护过的陌生人",
            "change_description": "认识少女",
            "event_cause": "出手相助",
            "evidence": "少女怔怔看着他"
        }),
    ];
    let suggestions = vec![
        json!({
            "chapter_number": 2,
            "key": "见义勇为",
            "value": "主动保护弱者",
            "evidence": "他挡在少女身前",
            "reasoning": "行为体现保护弱者。",
            "sequence": 0
        }),
        json!({
            "chapter_number": 1,
            "key": "谨慎",
            "value": "行动前观察",
            "evidence": "他先看向四周",
            "reasoning": "行为体现谨慎。",
            "sequence": 0
        }),
    ];
    let baseline_state = json!({
        "explicit_attributes": {"位置": "山门"},
        "implicit_attributes": {},
        "social_attributes": {}
    });

    let events = pending_character_sync_attribute_events(&baseline_state, &changes, &suggestions)
        .expect("events should build");

    let keys: Vec<_> = events.iter().map(|event| event.key.as_str()).collect();
    assert_eq!(keys, vec!["谨慎", "位置", "见义勇为", "少女"]);
    assert!(events[0].from_classification);
    assert_eq!(events[0].old_value, Value::Null);
    assert!(!events[1].from_classification);
    assert_eq!(events[1].old_value, json!("山门"));
    assert!(events[2].from_classification);
    assert_eq!(events[2].old_value, Value::Null);
}

#[test]
fn projection_applies_changes_in_given_chapter_order() {
    let changes = vec![
        json!({
            "chapter_number": 1,
            "category": "implicit",
            "key": "谨慎",
            "operation": "modify",
            "new_value": "观察后行动",
            "change_description": "首次变化",
            "event_cause": "第1章行动",
            "evidence": "原文1"
        }),
        json!({
            "chapter_number": 2,
            "category": "implicit",
            "key": "谨慎",
            "operation": "modify",
            "new_value": "果断行动",
            "change_description": "后续变化",
            "event_cause": "第2章行动",
            "evidence": "原文2"
        }),
    ];

    let projected = project_character_attribute_changes(
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {"谨慎": "谨慎"},
            "social_attributes": {}
        }),
        &changes,
    )
    .expect("projection should succeed");

    assert_eq!(projected["implicit_attributes"]["谨慎"], json!("果断行动"));
}
