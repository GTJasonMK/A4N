use serde_json::json;

use a4n_api_contract::blueprint::StartBlueprintGenerationInput;

#[test]
fn start_blueprint_generation_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<StartBlueprintGenerationInput>(json!({
        "meta": {
            "start_request_id": "test-start",
            "retry_of_task_id": null
        },
        "project_id": 7,
        "title": "标题",
        "genre": "类型",
        "target_length": 30,
        "projectId": 7
    }))
    .expect_err("unknown fields must fail instead of being ignored");

    assert!(err.to_string().contains("unknown field"));
}
