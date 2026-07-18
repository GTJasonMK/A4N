use serde_json::json;

use super::shared::i64_arg_or_system_default;

#[test]
fn config_id_accepts_system_default_and_positive_ids() {
    assert_eq!(
        i64_arg_or_system_default(&json!({"configId": -1}), "configId").unwrap(),
        -1
    );
    assert_eq!(
        i64_arg_or_system_default(&json!({"configId": 7}), "configId").unwrap(),
        7
    );
}

#[test]
fn config_id_rejects_zero_negative_and_non_integer_values() {
    for value in [json!(0), json!(-2), json!("1"), json!(null)] {
        assert!(i64_arg_or_system_default(&json!({"configId": value}), "configId").is_err());
    }
}
