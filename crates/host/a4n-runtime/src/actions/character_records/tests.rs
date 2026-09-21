use a4n_api_contract::character::ListSnapshotsInput;
use serde_json::json;

#[test]
fn list_snapshots_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<ListSnapshotsInput>(json!({
        "profile_id": 1,
        "start_chapter": null,
        "unexpected": true
    }))
    .expect_err("unknown fields should be rejected");

    assert!(err.to_string().contains("unknown field"));
}
