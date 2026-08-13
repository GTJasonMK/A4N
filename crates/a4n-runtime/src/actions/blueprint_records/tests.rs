use serde_json::json;

use a4n_api_contract::blueprint::StartBlueprintGenerationInput;
use a4n_core::models::project::blueprint_version;
use chrono::NaiveDate;

use super::projections::blueprint_version_summary;

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

#[test]
fn blueprint_version_summary_rejects_missing_or_noncanonical_metadata() {
    let version = blueprint_version::Model {
        id: 7,
        project_id: 3,
        version: "1.0".to_string(),
        change_description: Some("自动生成首版蓝图".to_string()),
        blueprint_data: json!({}),
        created_at: NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        created_by: Some("system".to_string()),
    };
    let summary = blueprint_version_summary(&version).unwrap();
    assert_eq!(summary.change_description, "自动生成首版蓝图");
    assert_eq!(summary.created_by, "system");

    for (change_description, created_by, expected) in [
        (None, version.created_by.clone(), "缺少 change_description"),
        (
            Some(" 自动生成首版蓝图 ".to_string()),
            version.created_by.clone(),
            "change_description 非规范",
        ),
        (version.change_description.clone(), None, "缺少 created_by"),
        (
            version.change_description.clone(),
            Some(" system ".to_string()),
            "created_by 非规范",
        ),
    ] {
        let mut invalid = version.clone();
        invalid.change_description = change_description;
        invalid.created_by = created_by;
        assert!(
            blueprint_version_summary(&invalid)
                .err()
                .unwrap()
                .to_string()
                .contains(expected)
        );
    }
}
