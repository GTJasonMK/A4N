use serde_json::{Value, json};

use super::validate_reverse_part_outline_items;

fn part(part_number: i32, start_chapter: i32, end_chapter: i32) -> Value {
    json!({
        "part_number": part_number,
        "title": format!("第{part_number}部"),
        "start_chapter": start_chapter,
        "end_chapter": end_chapter,
        "summary": "分部摘要",
        "theme": "代价",
        "key_events": ["旧敌回归", "盟友背叛", "真相浮出"],
        "character_focus": {"主角": "从迟疑转为主动"}
    })
}

#[test]
fn rejects_overlapping_ranges() {
    let parts = vec![part(1, 1, 10), part(2, 10, 20)];

    let err = validate_reverse_part_outline_items(&parts)
        .expect_err("overlapping chapter ranges must fail");

    assert!(err.contains("章节范围重叠"));
    assert!(err.contains("part 1(1-10)"));
    assert!(err.contains("part 2(10-20)"));
}

#[test]
fn rejects_gapped_ranges() {
    let parts = vec![part(1, 1, 10), part(2, 12, 20)];

    let err =
        validate_reverse_part_outline_items(&parts).expect_err("gapped chapter ranges must fail");

    assert!(err.contains("章节范围有缺口"));
    assert!(err.contains("第 11 章到第 11 章未覆盖"));
}

#[test]
fn accepts_any_key_events_array() {
    let mut item = part(1, 1, 10);
    item["key_events"] = json!(["唯一事件"]);

    validate_reverse_part_outline_items(&[item])
        .expect("apply contract only requires key_events to be an array");
}

#[test]
fn rejects_reversed_range() {
    let parts = vec![part(1, 10, 1)];

    let err = validate_reverse_part_outline_items(&parts)
        .expect_err("end chapter before start chapter must fail");

    assert!(err.contains("end_chapter 不能小于 start_chapter"));
}

#[test]
fn rejects_non_array_key_events() {
    let mut item = part(1, 1, 10);
    item["key_events"] = json!("旧敌回归");

    let err = validate_reverse_part_outline_items(&[item])
        .expect_err("key_events must keep its array contract");

    assert!(err.contains("key_events 必须是数组"));
}

#[test]
fn rejects_unknown_fields() {
    let mut item = part(1, 1, 10);
    item["total_chapters"] = json!(10);

    let err = validate_reverse_part_outline_items(&[item])
        .expect_err("unknown part outline fields must fail");

    assert!(err.contains("未知字段 total_chapters"));
}
