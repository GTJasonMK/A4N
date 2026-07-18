use serde_json::json;

use super::{collect_json_strings, stringify_json_value};

#[test]
fn collect_json_strings_respects_zero_limit_for_scalar_strings() {
    let value = json!("摘要");

    assert!(collect_json_strings(Some(&value), 0).is_empty());
}

#[test]
fn stringify_json_value_truncates_without_alternate_serialization() {
    let value = json!({"summary": "abcdef"});

    assert_eq!(stringify_json_value(&value, 10), "{\"summary\"…");
}
