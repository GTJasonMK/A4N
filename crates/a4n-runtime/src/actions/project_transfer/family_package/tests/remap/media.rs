use super::*;

#[test]
fn media_request_snapshot_remaps_package_local_ids() {
    let mut maps = ImportIdMaps::default();
    maps.generation_spec_ids.insert(1, 101);
    maps.prompt_artifact_ids.insert(2, 102);
    maps.assembly_timeline_ids.insert(3, 103);

    let remapped = remap_media_request_snapshot(
        Some(&json!({
            "config_id": 9,
            "generation_spec_id": 1,
            "prompt_artifact_id": 2,
            "timeline_id": 3,
            "provider_job_id": "remote-job"
        })),
        &maps,
    )
    .expect("media request snapshot ids should be remapped")
    .expect("request snapshot should remain present");

    assert_eq!(remapped["generation_spec_id"], json!(101));
    assert_eq!(remapped["prompt_artifact_id"], json!(102));
    assert_eq!(remapped["timeline_id"], json!(103));
    assert!(remapped.get("config_id").is_none());
    assert_eq!(remapped["provider_job_id"], json!("remote-job"));
}

#[test]
fn media_request_snapshot_remaps_galgame_audio_target_line() {
    let mut maps = ImportIdMaps::default();
    maps.galgame_line_ids.insert(10, 110);

    let remapped = remap_media_request_snapshot(
        Some(&json!({
            "source": "galgame_audio_import",
            "media_kind": "audio",
            "target_line_id": 10,
            "role": "bgm",
            "file_name": "theme.mp3"
        })),
        &maps,
    )
    .expect("galgame audio request snapshot ids should be remapped")
    .expect("request snapshot should remain present");

    assert_eq!(remapped["target_line_id"], json!(110));
    assert_eq!(remapped["role"], json!("bgm"));
}

#[test]
fn media_request_snapshot_remaps_galgame_target_entity_id() {
    let mut maps = ImportIdMaps::default();
    maps.galgame_scene_ids.insert(20, 120);
    maps.galgame_line_ids.insert(30, 130);

    let scene = remap_media_request_snapshot(
        Some(&json!({
            "source": "galgame_asset_generation",
            "target_entity_kind": "galgame_scene",
            "target_entity_id": 20
        })),
        &maps,
    )
    .expect("galgame scene request snapshot should be remapped")
    .expect("request snapshot should remain present");
    assert_eq!(scene["target_entity_id"], json!(120));

    let line = remap_media_request_snapshot(
        Some(&json!({
            "source": "galgame_asset_generation",
            "target_entity_kind": "galgame_line",
            "target_entity_id": 30
        })),
        &maps,
    )
    .expect("galgame line request snapshot should be remapped")
    .expect("request snapshot should remain present");
    assert_eq!(line["target_entity_id"], json!(130));
}
