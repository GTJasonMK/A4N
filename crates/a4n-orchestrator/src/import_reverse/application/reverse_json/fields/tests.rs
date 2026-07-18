use serde_json::json;

use super::{optional_i32, reject_unknown_keys, required_array, required_i32, required_string};

#[test]
fn required_string_rejects_missing_or_empty_values() {
    let value = json!({"title": "  "});

    assert!(required_string(&value, "title", "reverse_blueprint").is_err());
    assert!(required_string(&value, "genre", "reverse_blueprint").is_err());
}

#[test]
fn required_i32_does_not_parse_string_numbers() {
    let value = json!({"chapter_number": "1"});

    assert!(required_i32(&value, "chapter_number", "reverse_outline").is_err());
}

#[test]
fn optional_i32_rejects_present_non_integer_values() {
    let value = json!({"total_chapters": "50"});

    assert!(optional_i32(&value, "total_chapters", "reverse_blueprint").is_err());
}

#[test]
fn required_array_requires_canonical_key() {
    let value = json!({"chapter_outlines": []});

    assert!(required_array(&value, "outlines", "reverse_outline").is_err());
}

#[test]
fn reject_unknown_keys_rejects_extra_fields() {
    let value = json!({"outlines": [], "legacy": true});

    let err = reject_unknown_keys(&value, &["outlines"], "reverse_outline")
        .expect_err("unknown keys must fail");

    assert!(err.contains("未知字段 legacy"));
}
