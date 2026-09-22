use super::super::*;

pub(in crate::project::family::import::tests::remap) fn source_snapshot_outline_record(
    outline_kind: &str,
) -> source_snapshot_outline::Model {
    let payload = json!({
        "id": 301,
        "project_id": 1,
        "outline_kind": outline_kind,
        "title": "冻结大纲"
    });
    source_snapshot_outline::Model {
        id: 1201,
        snapshot_id: 901,
        root_project_id: 1,
        source_outline_id: 301,
        outline_kind: outline_kind.to_string(),
        payload_hash: hash_json(&payload),
        payload_json: payload,
    }
}

pub(in crate::project::family::import::tests::remap) fn source_snapshot_profile_record(
    profile_kind: &str,
) -> source_snapshot_profile::Model {
    let payload = json!({
        "id": 401,
        "project_id": 1,
        "profile_kind": profile_kind,
        "name": "冻结档案"
    });
    source_snapshot_profile::Model {
        id: 1301,
        snapshot_id: 901,
        root_project_id: 1,
        profile_kind: profile_kind.to_string(),
        source_profile_id: 401,
        payload_hash: hash_json(&payload),
        payload_json: payload,
    }
}
