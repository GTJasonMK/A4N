use super::*;
use a4n_core::error::AppError;
use serde_json::json;

#[test]
fn compute_hash_is_deterministic() {
    let data = json!({
        "name": "阿青",
        "traits": ["calm", "precise"],
        "level": 3
    });

    let first = Sha256SnapshotStore::compute_hash(&data).expect("hash");
    let second = Sha256SnapshotStore::compute_hash(&data).expect("hash");

    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
}

#[test]
fn compute_hash_distinguishes_snapshot_content() {
    let first = Sha256SnapshotStore::compute_hash(&json!({"level": 3})).expect("hash");
    let second = Sha256SnapshotStore::compute_hash(&json!({"level": 4})).expect("hash");

    assert_ne!(first, second);
}

#[test]
fn diff_character_snapshot_groups_attribute_changes() {
    let from = json!({
        "synced_chapter": 1,
        "state_summary": "初始",
        "attributes": {
            "explicit_attributes": {
                "appearance": "黑衣",
                "weapon": "短刀"
            },
            "implicit_attributes": {
                "personality": "谨慎"
            },
            "social_attributes": {
                "faction": "无"
            }
        },
        "changes_in_chapter": 2,
        "behaviors_in_chapter": 1
    });
    let to = json!({
        "synced_chapter": 2,
        "state_summary": "加入队伍",
        "attributes": {
            "explicit_attributes": {
                "appearance": "青衣",
                "age": "二十"
            },
            "implicit_attributes": {
                "personality": "主动谨慎"
            },
            "social_attributes": {
                "faction": "调查队"
            }
        },
        "changes_in_chapter": 3,
        "behaviors_in_chapter": 4
    });

    let delta = Sha256SnapshotStore::diff_json(&from, &to).expect("canonical snapshots diff");

    assert_eq!(
        delta["attributes"]["explicit"]["added"]["age"],
        json!("二十")
    );
    assert_eq!(
        delta["attributes"]["explicit"]["modified"]["appearance"]["from"],
        json!("黑衣")
    );
    assert_eq!(
        delta["attributes"]["explicit"]["deleted"]["weapon"],
        json!("短刀")
    );
    assert_eq!(
        delta["attributes"]["implicit"]["modified"]["personality"]["to"],
        json!("主动谨慎")
    );
    assert_eq!(
        delta["attributes"]["social"]["modified"]["faction"]["to"],
        json!("调查队")
    );
    assert_eq!(delta["summary"]["from_chapter"], json!(1));
    assert_eq!(delta["summary"]["to_chapter"], json!(2));
}

#[test]
fn diff_character_snapshot_rejects_missing_attribute_category() {
    let from = json!({
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        }
    });
    let to = json!({
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {}
        }
    });

    let err = Sha256SnapshotStore::diff_json(&from, &to)
        .expect_err("missing character snapshot category must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("social_attributes"));
}
