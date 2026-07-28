use serde_json::{Value, json};

use super::*;

#[test]
fn assembly_push_status_transition_has_one_owner() {
    assert_eq!(
        next_assembly_push_status("accept", ASSEMBLY_PUSH_STATUS_PENDING).unwrap(),
        ASSEMBLY_PUSH_STATUS_ACCEPTED
    );
    assert_eq!(
        next_assembly_push_status("undo", ASSEMBLY_PUSH_STATUS_ACCEPTED).unwrap(),
        ASSEMBLY_PUSH_STATUS_PENDING
    );
    assert!(next_assembly_push_status("reject", ASSEMBLY_PUSH_STATUS_ACCEPTED).is_err());
    assert!(next_assembly_push_status("unknown", ASSEMBLY_PUSH_STATUS_PENDING).is_err());
}

#[test]
fn metadata_update_rejects_clearing_system_fields() {
    let err = a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
        })),
        None,
        true,
    )
    .expect_err("system metadata must not be cleared");

    assert!(err.to_string().contains("不能清空"));
    assert!(err.to_string().contains("config_id"));
    assert!(err.to_string().contains("provider_request_snapshot"));
}

#[test]
fn metadata_update_rejects_removing_or_changing_system_fields() {
    let existing = json!({
        "config_id": 7,
        "provider_request_snapshot": { "request": "body" },
        "assembly_insert": { "sort_order": 1000.0 },
    });

    let missing = a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&existing),
        Some(&json!({
            "provider_request_snapshot": { "request": "body" },
            "assembly_insert": { "sort_order": 2000.0 },
        })),
        false,
    )
    .expect_err("missing config_id must be rejected");
    assert!(missing.to_string().contains("config_id"));

    let changed = a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&existing),
        Some(&json!({
            "config_id": 8,
            "provider_request_snapshot": { "request": "body" },
            "assembly_insert": { "sort_order": 2000.0 },
        })),
        false,
    )
    .expect_err("changed config_id must be rejected");
    assert!(changed.to_string().contains("config_id"));
}

#[test]
fn metadata_update_allows_business_fields_when_system_fields_are_preserved() {
    a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
            "shot_number": 1,
            "assembly_insert": { "sort_order": 1000.0 },
        })),
        Some(&json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
            "shot_number": 2,
            "assembly_insert": { "sort_order": 2000.0 },
        })),
        false,
    )
    .expect("business metadata fields should remain editable");
}

#[test]
fn provider_response_metadata_merges_only_into_objects() {
    let merged = a4n_db::video::merge_video_provider_response_metadata(
        Some(json!({
            "config_id": 7,
            "provider_request_snapshot": {"schema_version": 1}
        })),
        Some(json!({"job_id": "provider-job"})),
    )
    .expect("object metadata can accept provider response")
    .expect("merged metadata should exist");

    assert_eq!(merged["config_id"], json!(7));
    assert_eq!(
        merged["provider_response"],
        json!({"job_id": "provider-job"})
    );

    let created = a4n_db::video::merge_video_provider_response_metadata(
        None,
        Some(json!({"job_id": "provider-job"})),
    )
    .expect("missing metadata can be initialized")
    .expect("created metadata should exist");
    assert_eq!(
        created,
        json!({"provider_response": {"job_id": "provider-job"}})
    );

    let unchanged = a4n_db::video::merge_video_provider_response_metadata(
        Some(json!({"config_id": 7})),
        Some(Value::Null),
    )
    .expect("null provider metadata should be treated as missing")
    .expect("existing metadata should be preserved");
    assert_eq!(unchanged, json!({"config_id": 7}));

    let empty = a4n_db::video::merge_video_provider_response_metadata(None, Some(Value::Null))
        .expect("null provider metadata should not create fake provider_response");
    assert_eq!(empty, None);

    let err = a4n_db::video::merge_video_provider_response_metadata(
        Some(json!("corrupted metadata")),
        Some(json!({"job_id": "provider-job"})),
    )
    .expect_err("non-object metadata must not be wrapped into previous");
    assert_eq!(
        err.to_string(),
        "视频 metadata 必须是对象，不能写入 provider_response"
    );
}
