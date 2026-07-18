use serde_json::json;

use super::{
    dimension_fields_section, format_json_for_rag, optional_json_text, required_rag_text,
    short_rag_title,
};

#[test]
fn json_record_text_formats_named_objects_and_arrays_stably() {
    let text = format_json_for_rag(&json!([
        {"name": "林舟", "current_goal": "寻找星门"},
        {"title": "旧城", "description": "废弃广播站"}
    ]));

    assert!(text.contains("1. 林舟"));
    assert!(text.contains("目标: 寻找星门"));
    assert!(text.contains("2. 旧城"));
    assert!(text.contains("描述: 废弃广播站"));
}

#[test]
fn record_text_rules_trim_require_and_truncate() {
    assert_eq!(
        required_rag_text(Some("  星门  "), "missing").unwrap(),
        "星门"
    );
    assert!(required_rag_text(Some("  "), "missing").is_err());
    assert_eq!(optional_json_text(Some(&json!(null))), None);
    assert_eq!(short_rag_title("  一二三四五  ", 3), "一二三...");
}

#[test]
fn dimension_section_requires_fields_unless_identity_is_explicit() {
    assert!(dimension_fields_section("角色", "state", "状态", "林舟", &[], false).is_none());
    let section = dimension_fields_section("角色", "identity", "身份", "林舟", &[], true)
        .expect("identity-only section");
    assert_eq!(section.dimension, "identity");
    assert_eq!(section.text, "[角色-身份]\n角色: 林舟");
}
