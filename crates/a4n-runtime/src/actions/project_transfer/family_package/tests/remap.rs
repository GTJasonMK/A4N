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

    assert!(err.contains("target 引用类型无效"));
    assert!(err.contains("conflict_scope=generation_spec"));
    assert!(err.contains("target_kind=image"));
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
                    "scene_segmentation": {
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
        stage_outputs["scene_segmentation"]["scenes"][0]["source_snapshot_chapter_id"],
        json!(122)
    );
    assert_eq!(
        stage_outputs["scene_segmentation"]["scenes"][0]["source_chapter_id"],
        json!(131)
    );
    assert_eq!(
        stage_outputs["scene_segmentation"]["scenes"][0]["source_span_ids"],
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
fn remap_generation_spec_params_rejects_galgame_portrait_without_variant_key() {
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
        "canonical_key": "galgame:10:portrait:60:expression:",
        "asset_kind": "portrait_expression",
        "character_profile_id": 60,
        "source_line_ids": [50]
    });

    let err = remap_generation_spec_params_json(&spec, &maps)
        .expect_err("galgame portrait params without expression/pose should be rejected");

    assert!(err.contains("缺少 expression"));
}

#[test]
fn remap_generation_spec_params_rejects_galgame_portrait_pose_without_expression() {
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
        "pose": "standing",
        "source_line_ids": [50]
    });

    let err = remap_generation_spec_params_json(&spec, &maps)
        .expect_err("galgame portrait params without expression should be rejected");

    assert!(err.contains("缺少 expression"));
}

#[test]
fn remap_snapshot_hash_json_rewrites_known_ids() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.chapter_ids.insert(10, 110);
    maps.character_profile_ids.insert(20, 120);
    maps.object_profile_ids.insert(30, 130);
    maps.scene_profile_ids.insert(40, 140);
    let character_payload = json!({"id": 20, "project_id": 1, "name": "角色"});
    let object_payload = json!({"id": 30, "project_id": 1, "name": "物体"});
    let scene_payload = json!({"id": 40, "project_id": 1, "name": "场景", "parent_scene_id": null});
    let snapshot_profiles = [
        source_snapshot_profile::Model {
            id: 1,
            snapshot_id: 901,
            root_project_id: 1,
            profile_kind: "character".to_string(),
            source_profile_id: 20,
            payload_hash: hash_json(&character_payload),
            payload_json: character_payload,
        },
        source_snapshot_profile::Model {
            id: 2,
            snapshot_id: 901,
            root_project_id: 1,
            profile_kind: "object".to_string(),
            source_profile_id: 30,
            payload_hash: hash_json(&object_payload),
            payload_json: object_payload,
        },
        source_snapshot_profile::Model {
            id: 3,
            snapshot_id: 901,
            root_project_id: 1,
            profile_kind: "scene".to_string(),
            source_profile_id: 40,
            payload_hash: hash_json(&scene_payload),
            payload_json: scene_payload,
        },
    ];
    let snapshot_profile_index = snapshot_profiles
        .iter()
        .map(|profile| {
            (
                (
                    profile.snapshot_id,
                    snapshot_profile_key(&profile.profile_kind, profile.source_profile_id),
                ),
                profile,
            )
        })
        .collect::<HashMap<_, _>>();

    let chapter_hashes = remap_chapter_hashes_json(
        &json!([{"source_chapter_id": 10, "content_hash": "a"}]),
        &maps,
    )
    .unwrap();
    let profile_hashes = remap_profile_hashes_json(
        &json!([
            {"kind": "character", "id": 20, "hash": "b"},
            {"kind": "object", "id": 30, "hash": "c"},
            {"kind": "scene", "id": 40, "hash": "d"}
        ]),
        901,
        &snapshot_profile_index,
        &maps,
    )
    .unwrap();

    assert_eq!(chapter_hashes[0]["source_chapter_id"], json!(110));
    assert_eq!(profile_hashes[0]["id"], json!(120));
    assert_eq!(profile_hashes[1]["id"], json!(130));
    assert_eq!(profile_hashes[2]["id"], json!(140));
    assert_eq!(
        profile_hashes[0]["hash"],
        json!(hash_json(
            &json!({"id": 120, "project_id": 101, "name": "角色"})
        ))
    );
    assert_eq!(
        profile_hashes[1]["hash"],
        json!(hash_json(
            &json!({"id": 130, "project_id": 101, "name": "物体"})
        ))
    );
    assert_eq!(
        profile_hashes[2]["hash"],
        json!(hash_json(
            &json!({"id": 140, "project_id": 101, "name": "场景", "parent_scene_id": null})
        ))
    );
}

#[test]
fn remap_source_snapshot_outline_payload_rewrites_payload_identity_and_hash() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.part_outline_ids.insert(301, 1301);
    maps.chapter_outline_ids.insert(302, 1302);

    let mut outline = source_snapshot_outline_record("chapter");
    outline.source_outline_id = 302;
    outline.payload_json = json!({
        "id": 302,
        "project_id": 1,
        "part_id": 301,
        "title": "冻结章纲"
    });
    outline.payload_hash = hash_json(&outline.payload_json);

    let (payload, payload_hash) = remap_source_snapshot_outline_payload(&outline, &maps).unwrap();

    assert_eq!(payload["id"], json!(1302));
    assert_eq!(payload["project_id"], json!(101));
    assert_eq!(payload["part_id"], json!(1301));
    assert_eq!(payload_hash, hash_json(&payload));
    assert_ne!(payload_hash, outline.payload_hash);
}

#[test]
fn remap_source_snapshot_profile_payload_rewrites_scene_parent_and_hash() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.scene_profile_ids.insert(401, 1401);
    maps.scene_profile_ids.insert(402, 1402);

    let mut profile = source_snapshot_profile_record("scene");
    profile.source_profile_id = 402;
    profile.payload_json = json!({
        "id": 402,
        "project_id": 1,
        "parent_scene_id": 401,
        "name": "子场景"
    });
    profile.payload_hash = hash_json(&profile.payload_json);

    let (payload, payload_hash) = remap_source_snapshot_profile_payload(&profile, &maps).unwrap();

    assert_eq!(payload["id"], json!(1402));
    assert_eq!(payload["project_id"], json!(101));
    assert_eq!(payload["parent_scene_id"], json!(1401));
    assert_eq!(payload_hash, hash_json(&payload));
    assert_ne!(payload_hash, profile.payload_hash);
}
