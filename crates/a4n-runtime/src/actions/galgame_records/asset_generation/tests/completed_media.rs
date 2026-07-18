#[test]
fn completed_visual_media_request_accepts_galgame_asset_generation_contract() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": { "id": 100, "title": "雨夜车站" },
            "source": {
                "source_snapshot_chapter_id": 40,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");

    validate_galgame_visual_media_request_snapshot(
        &spec,
        &plan,
        701,
        Some(&json!({
            "source": "galgame_asset_generation",
            "target_kind": "galgame_background",
            "target_entity_kind": "galgame_scene",
            "target_entity_id": 100
        })),
    )
    .expect("valid completed visual media provenance should pass");
}

#[test]
fn completed_visual_media_request_rejects_missing_snapshot() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": { "id": 100, "title": "雨夜车站" },
            "source": {
                "source_snapshot_chapter_id": 40,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");

    let err = validate_galgame_visual_media_request_snapshot(&spec, &plan, 701, None)
        .expect_err("completed visual media provenance must keep request snapshot");

    assert!(err.contains("缺少 request_snapshot_json"));
    assert!(err.contains("media_asset_id=701"));
}

#[test]
fn completed_visual_media_request_rejects_wrong_source() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": { "id": 1000, "text": "她回过头。" },
            "scene": { "id": 100, "title": "雨夜车站" },
            "cg_note": "两人在雨中对视"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");

    let err = validate_galgame_visual_media_request_snapshot(
        &spec,
        &plan,
        702,
        Some(&json!({
            "source": "manual_binding",
            "target_kind": "galgame_cg",
            "target_entity_kind": "galgame_line",
            "target_entity_id": 1000
        })),
    )
    .expect_err("completed visual media provenance source must be Galgame asset generation");

    assert!(err.contains("媒体来源 source 无效"));
    assert!(err.contains("manual_binding"));
}

#[test]
fn completed_visual_media_request_rejects_target_mismatch() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": { "id": 1000, "text": "她回过头。" },
            "scene": { "id": 100, "title": "雨夜车站" },
            "cg_note": "两人在雨中对视"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");

    let err = validate_galgame_visual_media_request_snapshot(
        &spec,
        &plan,
        702,
        Some(&json!({
            "source": "galgame_asset_generation",
            "target_kind": "galgame_cg",
            "target_entity_kind": "galgame_line",
            "target_entity_id": 999
        })),
    )
    .expect_err("completed visual media provenance target must match the spec");

    assert!(err.contains("target_entity_id 与 spec 目标不一致"));
    assert!(err.contains("actual=999"));
    assert!(err.contains("expected=1000"));
}
