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
    assert!(remapped.get("provider_job_id").is_none());
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
fn manga_generated_image_metadata_and_source_paths_remap() {
    let mut maps = ImportIdMaps::default();
    maps.chapter_ids.insert(10, 110);
    maps.chapter_manga_prompt_ids.insert(20, 120);
    maps.generated_image_ids.insert(30, 130);
    maps.generated_image_paths
        .insert(30, "images/generated/130.png".to_string());
    maps.character_portrait_ids.insert(40, 140);
    maps.character_portrait_paths
        .insert(40, "images/portraits/140.png".to_string());

    let metadata = remap_manga_generated_image_metadata(
        &json!({
            "config_id": 99,
            "page_prompt_id": 20,
            "reference_bindings": [{
                "character_name": "hero",
                "bound_asset": {
                    "asset_kind": "character_portrait",
                    "asset_id": 40,
                    "image_path": "old/portrait.png"
                }
            }, {
                "character_name": "mentor",
                "bound_asset": {
                    "asset_kind": "generated_image",
                    "asset_id": 30,
                    "image_path": "old/generated.png"
                }
            }],
            "generation": {"chapter_id": 10, "config_id": 98}
        }),
        &maps,
    )
    .unwrap();
    assert_eq!(metadata["page_prompt_id"], json!(120));
    assert_eq!(metadata["generation"]["chapter_id"], json!(110));
    assert!(metadata["generation"].get("config_id").is_none());
    assert_eq!(
        metadata["reference_bindings"][0]["bound_asset"],
        json!({
            "asset_kind": "character_portrait",
            "asset_id": 140,
            "image_path": "images/portraits/140.png"
        })
    );
    assert_eq!(
        metadata["reference_bindings"][1]["bound_asset"],
        json!({
            "asset_kind": "generated_image",
            "asset_id": 130,
            "image_path": "images/generated/130.png"
        })
    );
    assert!(metadata.get("config_id").is_none());
    assert_eq!(
        remap_media_source_external_path(Some("generated_image:30"), &maps).unwrap(),
        Some("generated_image:130".to_string())
    );
}

#[test]
fn manga_page_generation_v4_layout_remaps_project_documents_and_images() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(33, 133);
    maps.chapter_ids.insert(22, 122);
    maps.authoring_document_ids.insert(71, 171);
    maps.authoring_document_ids.insert(72, 172);
    maps.authoring_document_sha256s.insert(71, "d".repeat(64));
    maps.authoring_document_sha256s.insert(72, "b".repeat(64));
    maps.generated_image_ids.insert(30, 130);
    maps.generated_image_paths
        .insert(30, "images/generated/130.png".to_string());

    let remapped = remap_manga_page_layout(
        &json!({
            "schema_version": "manga_page_generation_v4",
            "page_id": "page-0001",
            "page_number": 1,
            "page_plan_ref": {"id": 72, "revision": 2, "sha256": "a".repeat(64)},
            "pipeline": {
                "schema_version": "manga_page_pipeline_state_v1",
                "page_id": "page-0001",
                "status": "pending",
                "stage": "adaptation",
                "attempt": 1,
                "source_fingerprint": "content-sha256",
                "stage_fingerprints": {},
                "page_content": {
                    "summary": "林岚进入旧车站。",
                    "source_span_ids": [],
                    "beat_refs": [{"unit_id": "station", "beat_id": "arrival"}],
                    "action": "核对怀表。",
                    "reaction": null,
                    "dialogue_intent": null,
                    "entry_state": "站在入口。",
                    "exit_state": "进入通道。"
                },
                "page_adaptation": null,
                "storyboard": null,
                "drawing_prompt": null,
                "image": null,
                "error": null
            },
            "reference_bindings": [{
                "character_name": "林岚",
                "bound_asset": {
                    "asset_kind": "generated_image",
                    "asset_id": 30,
                    "image_path": "images/generated/old.png"
                }
            }],
            "prompt_fingerprint": {},
            "source": {
                "root_project_id": 33,
                "source_chapter_id": 22,
                "source_snapshot_id": null,
                "source_snapshot_chapter_id": null,
                "source_revision_key": "live:22:content-sha256",
                "content_sha256": "content-sha256",
                "adaptation_plan_id": 71,
                "adaptation_plan_revision": 1,
                "adaptation_plan_sha256": "c".repeat(64)
            }
        }),
        &maps,
        1,
        "manga page",
    )
    .expect("v4 manga layouts should be remapped without downgrading their schema");

    assert_eq!(
        remapped["schema_version"],
        json!("manga_page_generation_v4")
    );
    assert_eq!(remapped["page_id"], json!("page-0001"));
    assert_eq!(remapped["page_plan_ref"]["id"], json!(172));
    assert_eq!(remapped["page_plan_ref"]["sha256"], json!("b".repeat(64)));
    assert_eq!(remapped["source"]["root_project_id"], json!(133));
    assert_eq!(remapped["source"]["source_chapter_id"], json!(122));
    assert_eq!(
        remapped["source"]["source_revision_key"],
        json!("live:122:content-sha256")
    );
    assert_eq!(
        remapped["reference_bindings"][0]["bound_asset"]["asset_id"],
        json!(130)
    );
    assert_eq!(
        remapped["reference_bindings"][0]["bound_asset"]["image_path"],
        json!("images/generated/130.png")
    );
}

#[test]
fn media_request_snapshot_rejects_target_id_without_kind() {
    let error =
        remap_media_request_snapshot(Some(&json!({"target_entity_id": 20})), &Default::default())
            .expect_err("target id without target kind must fail");

    assert!(error.to_string().contains("target_entity_kind"));
}

#[test]
fn imported_video_fingerprints_follow_remapped_resources_without_rebinding_stale_outputs() {
    let mut maps = ImportIdMaps::default();
    maps.video_segment_fingerprints
        .insert("old-segment".into(), "new-segment".into());
    maps.video_recipe_fingerprints
        .insert("old-recipe".into(), "new-recipe".into());
    let remapped = remap_video_metadata(Some(&json!({
        "video_execution_identity": {
            "segment_sha256": "old-segment", "segment_recipe_fingerprint": "old-recipe",
            "segment_identities": [
                {"source_shot_id": "shot-1", "segment_index_in_shot": 1, "segment_sha256": "old-segment"},
                {"source_shot_id": "shot-2", "segment_index_in_shot": 1, "segment_sha256": "stale-segment"}
            ]
        }
    })), &maps).unwrap().unwrap();
    let identity = &remapped["video_execution_identity"];
    assert_eq!(identity["segment_sha256"], "new-segment");
    assert_eq!(identity["segment_recipe_fingerprint"], "new-recipe");
    assert_eq!(
        identity["segment_identities"][0]["segment_sha256"],
        "new-segment"
    );
    assert_eq!(
        identity["segment_identities"][1]["segment_sha256"],
        "stale-segment"
    );
}

#[test]
fn video_refs_remap_metadata_provenance_binding_and_runtime_identity() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.chapter_ids.insert(2, 102);
    maps.chapter_video_prompt_ids.insert(3, 103);
    maps.generated_image_ids.insert(4, 104);
    maps.generated_video_ids.insert(5, 105);

    let metadata = remap_video_metadata(
        Some(&json!({
            "prompt_id": 3,
            "source_image_id": 4,
            "source_video_id": 5,
            "reference_image_ids": [4],
            "workflow_run_id": 9,
            "provider_job_id": "job",
            "config_id": 7
        })),
        &maps,
    )
    .unwrap()
    .unwrap();
    assert_eq!(metadata["prompt_id"], json!(103));
    assert_eq!(metadata["source_image_id"], json!(104));
    assert_eq!(metadata["source_video_id"], json!(105));
    assert_eq!(metadata["reference_image_ids"], json!([104]));
    for key in ["workflow_run_id", "provider_job_id", "config_id"] {
        assert!(metadata.get(key).is_none());
    }

    let request = remap_media_request_snapshot(
        Some(&json!({
            "chapter_id": 2,
            "generated_video_id": 5,
            "generated_image_id": 4,
            "target_entity_kind": "chapter_video_prompt",
            "target_entity_id": 3
        })),
        &maps,
    )
    .unwrap()
    .unwrap();
    assert_eq!(request["chapter_id"], json!(102));
    assert_eq!(request["generated_video_id"], json!(105));
    assert_eq!(request["generated_image_id"], json!(104));
    assert_eq!(request["target_entity_id"], json!(103));

    let binding = media_binding::Model {
        id: 1,
        media_asset_id: 2,
        target_kind: "generated_video".to_string(),
        target_id: 5,
        role: "generated_result".to_string(),
        created_at: dt(),
    };
    assert_eq!(remap_media_binding_target_id(&binding, &maps).unwrap(), 105);
    assert_eq!(
        remap_media_source_external_path(Some("generated_video:5"), &maps).unwrap(),
        Some("generated_video:105".to_string())
    );
}
