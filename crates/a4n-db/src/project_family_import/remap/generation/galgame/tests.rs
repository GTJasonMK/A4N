use super::{
    remap_galgame_background_params, remap_galgame_cg_params, remap_galgame_portrait_params,
};
use crate::project_family_import::ImportIdMaps;
use a4n_core::models::generation::generation_spec;
use chrono::Utc;
use serde_json::{Value, json};

fn spec(project_id: i64, target_entity_id: i64) -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    generation_spec::Model {
        id: 1,
        project_id,
        root_project_id: project_id,
        source_context_kind: None,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "test".to_string(),
        target_entity_kind: "test".to_string(),
        target_entity_id: Some(target_entity_id),
        status: "pending".to_string(),
        source_version_hash: "hash".to_string(),
        params_json: json!({}),
        created_at: now,
        updated_at: now,
    }
}

fn object(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().cloned().expect("object fixture")
}

#[test]
fn background_remap_updates_canonical_scene_and_source_ids() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.galgame_scene_ids.insert(2, 202);
    maps.source_snapshot_chapter_ids.insert(3, 303);
    maps.chapter_ids.insert(4, 404);
    let mut params = object(json!({
        "scene": {"id": 2},
        "source": {"source_snapshot_chapter_id": 3, "source_chapter_id": 4}
    }));

    remap_galgame_background_params(&spec(1, 2), &mut params, &maps).unwrap();

    assert_eq!(
        params["canonical_key"],
        json!("galgame:101:background:scene:202")
    );
    assert_eq!(params["scene"]["id"], json!(202));
    assert_eq!(params["source"]["source_snapshot_chapter_id"], json!(303));
    assert_eq!(params["source"]["source_chapter_id"], json!(404));
}

#[test]
fn cg_remap_updates_line_speaker_scene_and_canonical_key() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.galgame_line_ids.insert(2, 202);
    maps.character_profile_ids.insert(3, 303);
    maps.galgame_scene_ids.insert(4, 404);
    let mut params = object(json!({
        "line": {"id": 2, "speaker_profile_id": 3},
        "scene": {"id": 4}
    }));

    remap_galgame_cg_params(&spec(1, 2), &mut params, &maps).unwrap();

    assert_eq!(params["canonical_key"], json!("galgame:101:cg:line:202"));
    assert_eq!(params["line"]["id"], json!(202));
    assert_eq!(params["line"]["speaker_profile_id"], json!(303));
    assert_eq!(params["scene"]["id"], json!(404));
}

#[test]
fn portrait_remap_updates_profile_lines_and_variant_key() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.character_profile_ids.insert(2, 202);
    maps.galgame_line_ids.insert(3, 303);
    maps.galgame_line_ids.insert(4, 404);
    let mut params = object(json!({
        "character_profile_id": 2,
        "source_line_ids": [3, 4],
        "expression": "开心",
        "pose": "挥手"
    }));

    remap_galgame_portrait_params(&spec(1, 2), &mut params, &maps).unwrap();

    assert_eq!(params["character_profile_id"], json!(202));
    assert_eq!(params["source_line_ids"], json!([303, 404]));
    assert_eq!(
        params["canonical_key"],
        json!("galgame:101:portrait:202:expression:开心:pose:挥手")
    );
}

#[test]
fn portrait_remap_requires_non_empty_expression() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.character_profile_ids.insert(2, 202);
    let mut params = object(json!({"character_profile_id": 2, "expression": "  "}));

    let error = remap_galgame_portrait_params(&spec(1, 2), &mut params, &maps).unwrap_err();

    assert_eq!(
        error.to_string(),
        "Galgame 立绘差分 params_json 缺少 expression"
    );
}

#[test]
fn portrait_remap_requires_non_empty_pose() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.character_profile_ids.insert(2, 202);
    let mut params = object(json!({"character_profile_id": 2, "expression": "smile"}));

    let error = remap_galgame_portrait_params(&spec(1, 2), &mut params, &maps).unwrap_err();

    assert_eq!(error.to_string(), "Galgame 立绘差分 params_json 缺少 pose");
}
