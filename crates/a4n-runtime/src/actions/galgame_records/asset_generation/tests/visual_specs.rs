#[test]
fn visual_spec_params_accept_background_contract() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "source": {
                "source_snapshot_chapter_id": 40,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
    let context = background_target_context();

    validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect("background params should match target context");
}

#[test]
fn visual_spec_params_accept_cg_contract() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": {
                "id": 1000,
                "text": "她回过头。"
            },
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "cg_note": "两人在雨中对视",
            "cg_importance": "critical"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let context = cg_target_context();

    validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect("cg params should match target context");
}

#[test]
fn visual_spec_params_rejects_background_scene_id_mismatch() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": {
                "id": 200,
                "title": "雨夜车站"
            },
            "source": {
                "source_snapshot_chapter_id": 40,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
    let context = background_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("scene.id mismatch should fail");

    assert!(err.contains("scene.id 与目标不一致"));
    assert!(err.contains("actual=200"));
    assert!(err.contains("expected=100"));
}

#[test]
fn visual_spec_params_rejects_cg_line_id_mismatch() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": {
                "id": 999,
                "text": "她回过头。"
            },
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "cg_note": "两人在雨中对视"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let context = cg_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("line.id mismatch should fail");

    assert!(err.contains("line.id 与目标不一致"));
    assert!(err.contains("actual=999"));
    assert!(err.contains("expected=1000"));
}

#[test]
fn visual_spec_params_rejects_cg_scene_id_mismatch() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": {
                "id": 1000,
                "text": "她回过头。"
            },
            "scene": {
                "id": 200,
                "title": "另一场"
            },
            "cg_note": "两人在雨中对视"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let context = cg_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("scene.id mismatch should fail");

    assert!(err.contains("scene.id 与目标不一致"));
    assert!(err.contains("actual=200"));
    assert!(err.contains("expected=100"));
}

#[test]
fn visual_spec_params_rejects_cg_missing_note() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": {
                "id": 1000,
                "text": "她回过头。"
            },
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let context = cg_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("missing cg_note should fail");

    assert!(err.contains("cg_note 缺失"));
}

#[test]
fn visual_spec_params_rejects_invalid_cg_importance() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:1000",
            "line": {
                "id": 1000,
                "text": "她回过头。"
            },
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "cg_note": "两人在雨中对视",
            "cg_importance": "major"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let context = cg_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("invalid cg_importance should fail");

    assert!(err.contains("cg_importance"));
    assert!(err.contains("major"));
}

#[test]
fn visual_spec_params_rejects_source_context_mismatch() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(41),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "source": {
                "source_snapshot_chapter_id": 40,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
    let context = background_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("spec source context mismatch should fail");

    assert!(err.contains("源章节上下文与目标不一致"));
    assert!(err.contains("spec_source_snapshot_chapter_id=Some(41)"));
}

#[test]
fn visual_spec_params_rejects_background_source_params_mismatch() {
    let spec = background_visual_spec(
        10,
        Some(100),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:background:scene:100",
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "source": {
                "source_snapshot_chapter_id": 41,
                "source_chapter_id": 30
            }
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("background visual plan");
    let context = background_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("params source context mismatch should fail");

    assert!(err.contains("source.source_snapshot_chapter_id 与目标不一致"));
    assert!(err.contains("actual=41"));
    assert!(err.contains("expected=40"));
}

#[test]
fn visual_spec_params_rejects_canonical_key_mismatch() {
    let spec = cg_visual_spec(
        10,
        Some(1000),
        Some(30),
        Some(40),
        json!({
            "canonical_key": "galgame:10:cg:line:999",
            "line": {
                "id": 1000,
                "text": "她回过头。"
            },
            "scene": {
                "id": 100,
                "title": "雨夜车站"
            },
            "cg_note": "两人在雨中对视"
        }),
    );
    let plan = GalgameVisualSpecPlan::from_spec(&spec).expect("cg visual plan");
    let context = cg_target_context();

    let err = validate_galgame_visual_spec_params(&spec, &plan, &context)
        .expect_err("canonical key mismatch should fail");

    assert!(err.contains("canonical_key 与目标不一致"));
    assert!(err.contains("galgame:10:cg:line:999"));
    assert!(err.contains("galgame:10:cg:line:1000"));
}
