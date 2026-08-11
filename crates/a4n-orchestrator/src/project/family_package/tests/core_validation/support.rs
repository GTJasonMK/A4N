use super::*;

pub(super) fn galgame_background_params(
    project_id: i64,
    scene_id: i64,
    source_snapshot_chapter_id: i64,
    source_chapter_id: i64,
) -> Value {
    json!({
        "canonical_key": format!("galgame:{project_id}:background:scene:{scene_id}"),
        "asset_kind": "background",
        "scene": {
            "id": scene_id,
            "index": 1,
            "title": "开场"
        },
        "source": {
            "source_snapshot_chapter_id": source_snapshot_chapter_id,
            "source_chapter_id": source_chapter_id,
            "chapter_number": 1
        }
    })
}

pub(super) fn galgame_cg_params(
    project_id: i64,
    line_id: i64,
    scene_id: i64,
    speaker_profile_id: Option<i64>,
) -> Value {
    json!({
        "canonical_key": format!("galgame:{project_id}:cg:line:{line_id}"),
        "asset_kind": "event_cg",
        "line": {
            "id": line_id,
            "line_index": 1,
            "line_type": "dialogue",
            "speaker_profile_id": speaker_profile_id,
            "text": "这里是开场对白。"
        },
        "scene": {
            "id": scene_id,
            "title": "开场"
        },
        "cg_note": "开场特写",
        "cg_importance": "normal"
    })
}

pub(super) fn galgame_portrait_params(
    project_id: i64,
    profile_id: i64,
    expression: &str,
    pose: &str,
    source_line_ids: &[i64],
) -> Value {
    let variant_key = format!("expression:{expression}:pose:{pose}");
    json!({
        "canonical_key": format!("galgame:{project_id}:portrait:{profile_id}:{variant_key}"),
        "asset_kind": "portrait_expression",
        "character_profile_id": profile_id,
        "expression": expression,
        "pose": pose,
        "position_samples": ["left"],
        "source_line_ids": source_line_ids
    })
}

pub(super) fn add_frozen_character_profile(package: &mut ProjectFamilyPackage, profile_id: i64) {
    package.text_core.character_profiles = vec![character_profile_record(profile_id)];
    let mut snapshot_profile = source_snapshot_profile_record("character");
    snapshot_profile.source_profile_id = profile_id;
    snapshot_profile.payload_json["id"] = json!(profile_id);
    snapshot_profile.payload_hash = hash_json(&snapshot_profile.payload_json);
    package.source_snapshots[0].profile_hashes_json =
        Value::Array(vec![source_snapshot_profile_hash_row(&snapshot_profile)]);
    package.source_snapshot_profiles = vec![snapshot_profile];
}
