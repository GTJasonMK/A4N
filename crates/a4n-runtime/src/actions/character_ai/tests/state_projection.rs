use super::prelude::*;
use serde_json::json;

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
