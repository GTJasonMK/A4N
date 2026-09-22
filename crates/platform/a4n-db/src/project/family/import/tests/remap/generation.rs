use super::*;

#[test]
fn remap_generation_spec_source_version_hash_uses_imported_snapshot_hash() {
    let mut maps = ImportIdMaps::default();
    maps.source_snapshot_version_hashes
        .insert(901, "new-snapshot-hash".to_string());
    let mut spec = generation_spec_for_source_snapshot(901);
    spec.source_version_hash = "old-package-hash".to_string();

    let hash = remap_generation_spec_source_version_hash(&spec, &maps)
        .expect("generation spec should use the imported source snapshot hash");

    assert_eq!(hash, "new-snapshot-hash");
}

#[test]
fn remap_generation_spec_params_rewrites_galgame_background_ids() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(10, 110);
    maps.galgame_scene_ids.insert(20, 120);
    maps.source_snapshot_chapter_ids.insert(30, 130);
    maps.chapter_ids.insert(40, 140);

    let mut spec = generation_spec_record();
    spec.project_id = 10;
    spec.target_kind = "galgame_background".to_string();
    spec.target_entity_kind = "galgame_scene".to_string();
    spec.target_entity_id = Some(20);
    spec.params_json = json!({
        "canonical_key": "galgame:10:background:scene:20",
        "asset_kind": "background",
        "scene": {
            "id": 20,
            "index": 1,
            "title": "旧场景"
        },
        "source": {
            "source_snapshot_chapter_id": 30,
            "source_chapter_id": 40,
            "chapter_number": 1
        }
    });

    let remapped = remap_generation_spec_params_json(&spec, &maps)
        .expect("galgame background params ids should be remapped");

    assert_eq!(
        remapped["canonical_key"],
        json!("galgame:110:background:scene:120")
    );
    assert_eq!(remapped["scene"]["id"], json!(120));
    assert_eq!(remapped["source"]["source_snapshot_chapter_id"], json!(130));
    assert_eq!(remapped["source"]["source_chapter_id"], json!(140));
    assert_eq!(remapped["scene"]["title"], json!("旧场景"));
}

#[test]
fn remap_generation_spec_params_rewrites_galgame_cg_ids() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(10, 110);
    maps.galgame_scene_ids.insert(20, 120);
    maps.galgame_line_ids.insert(50, 150);
    maps.character_profile_ids.insert(60, 160);

    let mut spec = generation_spec_record();
    spec.project_id = 10;
    spec.target_kind = "galgame_cg".to_string();
    spec.target_entity_kind = "galgame_line".to_string();
    spec.target_entity_id = Some(50);
    spec.params_json = json!({
        "canonical_key": "galgame:10:cg:line:50",
        "asset_kind": "event_cg",
        "line": {
            "id": 50,
            "speaker_profile_id": 60,
            "text": "旧台词"
        },
        "scene": {
            "id": 20,
            "title": "旧场景"
        }
    });

    let remapped = remap_generation_spec_params_json(&spec, &maps)
        .expect("galgame cg params ids should be remapped");

    assert_eq!(remapped["canonical_key"], json!("galgame:110:cg:line:150"));
    assert_eq!(remapped["line"]["id"], json!(150));
    assert_eq!(remapped["line"]["speaker_profile_id"], json!(160));
    assert_eq!(remapped["scene"]["id"], json!(120));
    assert_eq!(remapped["line"]["text"], json!("旧台词"));
}

#[test]
fn remap_generation_spec_params_rewrites_galgame_portrait_ids() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(10, 110);
    maps.character_profile_ids.insert(60, 160);
    maps.galgame_line_ids.insert(50, 150);
    maps.galgame_line_ids.insert(51, 151);

    let mut spec = generation_spec_record();
    spec.project_id = 10;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(60);
    spec.params_json = json!({
        "canonical_key": "galgame:10:portrait:60:expression:smile:pose:standing",
        "asset_kind": "portrait_expression",
        "character_profile_id": 60,
        "expression": "smile",
        "pose": "standing",
        "position_samples": ["center", "left"],
        "source_line_ids": [50, 51]
    });

    let remapped = remap_generation_spec_params_json(&spec, &maps)
        .expect("galgame portrait params ids should be remapped");

    assert_eq!(
        remapped["canonical_key"],
        json!("galgame:110:portrait:160:expression:smile:pose:standing")
    );
    assert_eq!(remapped["character_profile_id"], json!(160));
    assert_eq!(remapped["source_line_ids"], json!([150, 151]));
    assert_eq!(remapped["position_samples"], json!(["center", "left"]));
    assert_eq!(remapped["expression"], json!("smile"));
    assert_eq!(remapped["pose"], json!("standing"));
}

#[test]
fn remap_generation_spec_params_rejects_galgame_portrait_without_expression() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(10, 110);
    maps.character_profile_ids.insert(60, 160);
    maps.galgame_line_ids.insert(50, 150);

    let mut spec = generation_spec_record();
    spec.project_id = 10;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(60);
    spec.params_json = json!({
        "canonical_key": "galgame:10:portrait:60:expression::pose:standing",
        "asset_kind": "portrait_expression",
        "character_profile_id": 60,
        "pose": "standing",
        "source_line_ids": [50]
    });

    let err = remap_generation_spec_params_json(&spec, &maps)
        .expect_err("galgame portrait params without expression/pose should be rejected");

    assert!(err.to_string().contains("缺少 expression"));
}

#[test]
fn remap_generation_spec_params_rejects_galgame_portrait_without_pose() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(10, 110);
    maps.character_profile_ids.insert(60, 160);
    maps.galgame_line_ids.insert(50, 150);

    let mut spec = generation_spec_record();
    spec.project_id = 10;
    spec.target_kind = "portrait_expression".to_string();
    spec.target_entity_kind = "character_profile".to_string();
    spec.target_entity_id = Some(60);
    spec.params_json = json!({
        "canonical_key": "galgame:10:portrait:60:pose:standing",
        "asset_kind": "portrait_expression",
        "character_profile_id": 60,
        "expression": "smile",
        "source_line_ids": [50]
    });

    let err = remap_generation_spec_params_json(&spec, &maps)
        .expect_err("galgame portrait params without pose should be rejected");

    assert!(err.to_string().contains("缺少 pose"));
}
