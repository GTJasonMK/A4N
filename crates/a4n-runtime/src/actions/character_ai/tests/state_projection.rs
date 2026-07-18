use super::super::*;
use serde_json::{Value, json};

#[test]
fn apply_attribute_to_state_rejects_corrupt_state_shape() {
    let mut state = json!("invalid");
    let err = apply_attribute_to_state(
        &mut state,
        "explicit",
        "appearance",
        "modify",
        json!("白衣"),
    )
    .expect_err("non-object state must fail");

    assert!(err.to_string().contains("角色当前状态不是 JSON 对象"));
}

#[test]
fn apply_attribute_to_state_rejects_corrupt_category_shape() {
    let mut state = json!({
        "explicit_attributes": null,
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
fn pending_attribute_events_keep_chapter_and_stage_order() {
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
        "explicit_attributes": {
            "位置": "山门"
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let events = pending_attribute_events(&baseline_state, &changes, &suggestions)
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
fn project_attribute_changes_uses_chapter_ordered_events() {
    let mut changes = vec![
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
    ];
    sort_sync_items_by_chapter(&mut changes).expect("changes should sort by chapter");

    let projected = project_attribute_changes(
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {
                "谨慎": "谨慎"
            },
            "social_attributes": {}
        }),
        &changes,
    )
    .expect("projection should succeed");

    assert_eq!(projected["implicit_attributes"]["谨慎"], json!("果断行动"));
}

#[test]
fn folded_event_state_uses_chapter_order_before_id() {
    let mut events = vec![
        (3, 10, "implicit", "谨慎", "modify", json!("第3章状态")),
        (
            2,
            11,
            "implicit",
            "谨慎",
            "modify",
            json!("第2章后写入状态"),
        ),
    ];

    let state = fold_attribute_event_tuples(
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {
                "谨慎": "初始状态"
            },
            "social_attributes": {}
        }),
        &mut events,
        2,
    );

    assert_eq!(
        state["implicit_attributes"]["谨慎"],
        json!("第2章后写入状态")
    );
}
