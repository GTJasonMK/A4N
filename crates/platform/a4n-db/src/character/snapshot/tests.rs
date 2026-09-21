use super::*;
use serde_json::json;

#[test]
fn snapshot_state_contract_errors_are_structured() {
    let error = snapshot_attributes_from_data(
        &json!({
            "attributes": {
                "explicit_attributes": {},
                "implicit_attributes": {}
            }
        }),
        3,
    )
    .expect_err("missing social attributes must fail");

    assert!(matches!(
        error,
        CharacterDbError::InvalidState { current, operation }
            if current == "第 3 章角色快照 attributes.social_attributes 缺少或不是 JSON 对象"
                && operation == "读取角色快照属性"
    ));
}

#[test]
fn snapshot_content_hash_is_deterministic() {
    let data = json!({"attributes": {"explicit_attributes": {"age": "20"}}});
    assert_eq!(
        character_snapshot_content_hash(&data).unwrap(),
        character_snapshot_content_hash(&data).unwrap()
    );
}
