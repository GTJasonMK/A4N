use super::super::*;
use serde_json::json;

#[test]
fn attribute_counts_requires_all_category_objects() {
    let counts = attribute_counts(&json!({
        "explicit_attributes": {"身份": "侦探"},
        "implicit_attributes": {},
        "social_attributes": {"关系": "同伴"},
    }))
    .expect("complete attribute categories should count");

    assert_eq!(counts["explicit"], json!(1));
    assert_eq!(counts["implicit"], json!(0));
    assert_eq!(counts["social"], json!(1));

    let missing = attribute_counts(&json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
    }))
    .expect_err("missing social category should fail");
    assert!(missing.contains("角色当前状态缺少字段 social_attributes"));

    let corrupt = attribute_counts(&json!({
        "explicit_attributes": [],
        "implicit_attributes": {},
        "social_attributes": {},
    }))
    .expect_err("corrupt explicit category should fail");
    assert!(corrupt.contains("角色当前状态字段 explicit_attributes 必须是对象"));
}
