use serde_json::json;

use super::{
    ensure_object, optional_positive_i64, optional_string, parse_llm_json_object,
    positive_i64_array, positive_i64_array_schema, positive_i64_schema, reject_unknown_fields,
    required_array_value, required_object_value, required_string, string_array, string_array_value,
};

#[test]
fn object_and_unknown_field_contracts_are_strict() {
    assert_eq!(
        ensure_object(&json!([]), "root"),
        Err("root 必须是 JSON 对象".to_string())
    );
    assert_eq!(
        reject_unknown_fields(&json!({"extra": true}), "root", &["title"]),
        Err("root 包含未支持字段 `extra`；只允许输出：title".to_string())
    );
    assert!(ensure_object(&json!({"title": "x"}), "root").is_ok());
    assert!(reject_unknown_fields(&json!({"title": "x"}), "root", &["title"]).is_ok());
}

#[test]
fn generated_id_contracts_share_the_integer_range_and_encoding() {
    let scalar = positive_i64_schema();
    assert_eq!(scalar["type"], "integer");
    assert_eq!(scalar["minimum"], 1);
    assert_eq!(scalar["maximum"], i64::MAX);
    assert!(scalar["description"].as_str().unwrap().contains("不加引号"));
    for (non_empty, minimum) in [(true, 1), (false, 0)] {
        let array = positive_i64_array_schema(non_empty);
        assert_eq!(array["type"], "array");
        assert_eq!(array["items"], scalar);
        assert_eq!(array["minItems"], minimum);
    }
}

#[test]
fn parses_only_trimmed_json_objects() {
    let parsed =
        parse_llm_json_object("  {\"title\":\"x\"}  ", "result").expect("trimmed JSON object");
    assert_eq!(parsed["title"], "x");
    assert_eq!(
        parse_llm_json_object("[]", "result"),
        Err("result 必须是 JSON 对象".to_string())
    );
    assert!(parse_llm_json_object("```json\n{}\n```", "result").is_err());
}

#[test]
fn required_and_optional_scalar_contracts_preserve_trim_and_null_rules() {
    let value = json!({"text": "  hello ", "empty": " ", "id": 3, "none": null});
    assert_eq!(
        required_string(&value, "text", "root"),
        Ok("hello".to_string())
    );
    assert_eq!(
        required_string(&value, "empty", "root"),
        Err("root.empty 不能为空".to_string())
    );
    assert_eq!(optional_string(&value, "none", "root"), Ok(None));
    assert_eq!(optional_string(&value, "empty", "root"), Ok(None));
    assert_eq!(optional_positive_i64(&value, "id", "root"), Ok(Some(3)));
    assert_eq!(
        optional_positive_i64(&json!({"id": 0}), "id", "root"),
        Err("root.id 必须大于 0".to_string())
    );
}

#[test]
fn array_contracts_trim_deduplicate_preserve_order_and_reject_invalid_values() {
    let value = json!({"names": [" A ", "A", "", "B"], "ids": [2, 2, 1]});
    assert_eq!(
        string_array(&value, "names", "root", true),
        Ok(vec!["A".to_string(), "B".to_string()])
    );
    assert_eq!(
        string_array_value(&value, "names", "root", true),
        Ok(json!(["A", "B"]))
    );
    assert_eq!(
        positive_i64_array(&value, "ids", "root", true),
        Ok(vec![2, 1])
    );
    assert_eq!(
        positive_i64_array(&json!({"ids": [-1]}), "ids", "root", true),
        Err("root.ids[0] 必须大于 0".to_string())
    );
    assert!(required_array_value(&value, "names", "root").is_ok());
    assert!(required_object_value(&json!({"item": {}}), "item", "root").is_ok());
    assert_eq!(
        required_object_value(&json!({"item": []}), "item", "root"),
        Err("root.item 必须是 JSON 对象".to_string())
    );
}
