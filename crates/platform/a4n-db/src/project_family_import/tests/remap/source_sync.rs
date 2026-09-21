use super::*;

#[test]
fn source_sync_generation_spec_conflict_target_uses_generation_spec_map() {
    let mut maps = ImportIdMaps::default();
    maps.generation_spec_ids.insert(11, 111);
    maps.video_segment_ids.insert(11, 211);

    let remapped = remap_source_sync_target_ref("generation_spec", "segment", 11, &maps)
        .expect("generation_spec conflict target should remap through generation_spec_ids");

    assert_eq!(remapped, 111);
}

#[test]
fn source_sync_generation_spec_conflict_rejects_unknown_target_kind() {
    let mut maps = ImportIdMaps::default();
    maps.generation_spec_ids.insert(11, 111);

    let err = remap_source_sync_target_ref("generation_spec", "image", 11, &maps)
        .expect_err("generation_spec conflicts must not wildcard unknown target kinds");
    let message = err.to_string();

    assert!(message.contains("target 引用类型无效"));
    assert!(message.contains("conflict_scope=generation_spec"));
    assert!(message.contains("target_kind=image"));
}

#[test]
fn source_sync_decision_payload_remaps_create_replacement_ids() {
    let mut maps = ImportIdMaps::default();
    maps.generation_spec_ids.insert(1, 101);
    maps.text_anchor_ids.insert(3, 103);
    maps.character_profile_ids.insert(4, 104);
    maps.media_asset_ids.insert(5, 105);
    maps.video_segment_ids.insert(6, 106);
    maps.video_segment_ids.insert(7, 107);

    let remapped = remap_source_sync_decision_payload(
        "create_replacement",
        &json!({
            "generation_spec_id": 1,
            "text_anchor_refs": [
                { "text_anchor_id": 3, "role": "direct_content" }
            ],
            "entity_refs": [
                { "entity_kind": "character", "entity_id": 4, "role": "subject" }
            ],
            "media_refs": [
                { "media_asset_id": 5, "role": "first_frame" }
            ],
            "replacement_segment": {
                "previous_segment_id": 6,
                "next_segment_id": 7
            }
        }),
        &maps,
        "同步决策 decision_payload_json",
    )
    .expect("source sync decision payload ids should be remapped");

    assert_eq!(remapped["generation_spec_id"], json!(101));
    assert_eq!(
        remapped["text_anchor_refs"][0]["text_anchor_id"],
        json!(103)
    );
    assert_eq!(remapped["entity_refs"][0]["entity_id"], json!(104));
    assert_eq!(remapped["media_refs"][0]["media_asset_id"], json!(105));
    assert_eq!(
        remapped["replacement_segment"]["previous_segment_id"],
        json!(106)
    );
    assert_eq!(
        remapped["replacement_segment"]["next_segment_id"],
        json!(107)
    );
}

#[test]
fn source_sync_decision_payload_remaps_rebind_anchor_ids() {
    let mut maps = ImportIdMaps::default();
    maps.generation_spec_ids.insert(1, 101);
    maps.text_anchor_ids.insert(2, 102);
    maps.text_anchor_ids.insert(3, 103);

    let remapped = remap_source_sync_decision_payload(
        "rebind_anchor",
        &json!({
            "generation_spec_id": 1,
            "anchor_mappings": [
                {
                    "old_text_anchor_id": 2,
                    "new_text_anchor_id": 3
                }
            ]
        }),
        &maps,
        "同步决策 decision_payload_json",
    )
    .expect("rebind_anchor payload ids should be remapped");

    assert_eq!(remapped["generation_spec_id"], json!(101));
    assert_eq!(
        remapped["anchor_mappings"][0]["old_text_anchor_id"],
        json!(102)
    );
    assert_eq!(
        remapped["anchor_mappings"][0]["new_text_anchor_id"],
        json!(103)
    );
}

#[test]
fn source_sync_decision_payload_remaps_regenerate_scene_ids() {
    let mut maps = ImportIdMaps::default();
    maps.galgame_scene_ids.insert(9, 109);
    maps.source_snapshot_chapter_ids.insert(21, 121);
    maps.source_snapshot_chapter_ids.insert(22, 122);

    let remapped = remap_source_sync_decision_payload(
        "regenerate_scene",
        &json!({
            "galgame_scene_id": 9,
            "result": {
                "galgame_scene_id": 9,
                "old_source_snapshot_chapter_id": 21,
                "new_source_snapshot_chapter_id": 22,
                "old_line_count": 3,
                "new_line_count": 4,
                "deleted_generation_spec_count": 2,
                "deleted_portrait_generation_spec_count": 1,
                "resolved_related_conflict_count": 5
            }
        }),
        &maps,
        "同步决策 decision_payload_json",
    )
    .expect("regenerate_scene payload ids should be remapped");

    assert_eq!(remapped["galgame_scene_id"], json!(109));
    assert_eq!(remapped["result"]["galgame_scene_id"], json!(109));
    assert_eq!(
        remapped["result"]["old_source_snapshot_chapter_id"],
        json!(121)
    );
    assert_eq!(
        remapped["result"]["new_source_snapshot_chapter_id"],
        json!(122)
    );
    assert_eq!(remapped["result"]["old_line_count"], json!(3));
    assert_eq!(remapped["result"]["new_line_count"], json!(4));
    assert_eq!(
        remapped["result"]["deleted_generation_spec_count"],
        json!(2)
    );
    assert_eq!(
        remapped["result"]["deleted_portrait_generation_spec_count"],
        json!(1)
    );
    assert_eq!(
        remapped["result"]["resolved_related_conflict_count"],
        json!(5)
    );
}

#[test]
fn source_sync_decision_payload_remaps_regenerate_scene_stage_outputs() {
    let mut maps = ImportIdMaps::default();
    maps.galgame_scene_ids.insert(9, 109);
    maps.source_snapshot_chapter_ids.insert(21, 121);
    maps.source_snapshot_chapter_ids.insert(22, 122);
    maps.chapter_ids.insert(31, 131);
    maps.text_anchor_ids.insert(41, 141);
    maps.text_anchor_ids.insert(42, 142);
    maps.character_profile_ids.insert(51, 151);
    maps.scene_profile_ids.insert(61, 161);

    let remapped = remap_source_sync_decision_payload(
        "regenerate_scene",
        &json!({
            "galgame_scene_id": 9,
            "result": {
                "galgame_scene_id": 9,
                "old_source_snapshot_chapter_id": 21,
                "new_source_snapshot_chapter_id": 22,
                "stage_outputs": {
                    "shooting_script_projection": {
                        "scenes": [
                            {
                                "index": 1,
                                "title": "旧场",
                                "source_snapshot_chapter_id": 22,
                                "source_chapter_id": 31,
                                "source_span_ids": ["ch1:ta41", "ch1:ta42"]
                            }
                        ]
                    },
                    "character_staging": {
                        "scene_lines": [
                            {
                                "scene_index": 1,
                                "lines": [
                                    {
                                        "index": 1,
                                        "line_type": "dialogue",
                                        "speaker_profile_id": 51,
                                        "text": "台词",
                                        "source_span_ids": ["ch1:ta41"]
                                    }
                                ]
                            }
                        ]
                    },
                    "visual_planning": {
                        "backgrounds": [
                            {
                                "scene_index": 1,
                                "background_note": "背景",
                                "scene_profile_id": 61,
                                "source_span_ids": ["ch1:ta42"]
                            }
                        ]
                    }
                }
            }
        }),
        &maps,
        "同步决策 decision_payload_json",
    )
    .expect("regenerate_scene stage output ids should be remapped");

    assert_eq!(remapped["galgame_scene_id"], json!(109));
    let stage_outputs = &remapped["result"]["stage_outputs"];
    assert_eq!(
        stage_outputs["shooting_script_projection"]["scenes"][0]["source_snapshot_chapter_id"],
        json!(122)
    );
    assert_eq!(
        stage_outputs["shooting_script_projection"]["scenes"][0]["source_chapter_id"],
        json!(131)
    );
    assert_eq!(
        stage_outputs["shooting_script_projection"]["scenes"][0]["source_span_ids"],
        json!(["ch1:ta141", "ch1:ta142"])
    );
    assert_eq!(
        stage_outputs["character_staging"]["scene_lines"][0]["lines"][0]["speaker_profile_id"],
        json!(151)
    );
    assert_eq!(
        stage_outputs["character_staging"]["scene_lines"][0]["lines"][0]["source_span_ids"],
        json!(["ch1:ta141"])
    );
    assert_eq!(
        stage_outputs["visual_planning"]["backgrounds"][0]["scene_profile_id"],
        json!(161)
    );
    assert_eq!(
        stage_outputs["visual_planning"]["backgrounds"][0]["source_span_ids"],
        json!(["ch1:ta142"])
    );
}

#[test]
fn source_sync_conflict_affected_refs_remaps_old_snapshot_chapter_id() {
    let mut maps = ImportIdMaps::default();
    maps.source_snapshot_chapter_ids.insert(21, 121);
    maps.source_snapshot_chapter_ids.insert(22, 122);
    maps.galgame_scene_ids.insert(31, 131);

    let mut conflict = source_sync_conflict_record();
    conflict.affected_refs_json = json!({
        "old_source_snapshot_chapter_id": 21,
        "source_snapshot_chapter_id": 22,
        "galgame_scene_id": 31
    });

    let remapped = remap_conflict_affected_refs(&conflict, &maps)
        .expect("source sync conflict affected refs should remap snapshot chapter ids");

    assert_eq!(remapped["old_source_snapshot_chapter_id"], json!(121));
    assert_eq!(remapped["source_snapshot_chapter_id"], json!(122));
    assert_eq!(remapped["galgame_scene_id"], json!(131));
}
