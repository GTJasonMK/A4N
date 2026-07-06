use super::*;

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_run_mode() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.mode = "preview".to_string();
    package.source_sync_runs = vec![run];

    let err =
        validate_package_manifest(&package).expect_err("source sync run mode must use known modes");

    assert!(err.contains("源同步运行模式无效"));
    assert!(err.contains("source_sync_run_id=801"));
    assert!(err.contains("mode=preview"));
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_run_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.status = "failed".to_string();
    package.source_sync_runs = vec![run];

    let err = validate_package_manifest(&package)
        .expect_err("source sync run status must use known states");

    assert!(err.contains("源同步运行状态无效"));
    assert!(err.contains("source_sync_run_id=801"));
    assert!(err.contains("status=failed"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_run_status_mode_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.status = SYNC_RUN_STATUS_COMPLETED.to_string();
    package.source_sync_runs = vec![run];

    let err = validate_package_manifest(&package)
        .expect_err("completed source sync runs must have apply mode");

    assert!(err.contains("源同步运行状态与模式不一致"));
    assert!(err.contains("source_sync_run_id=801"));
    assert!(err.contains("status=completed"));
    assert!(err.contains("mode=dry_run"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_run_states() {
    for status in SYNC_RUN_STATUSES {
        let mut package = package_with_part_outlines(Vec::new());
        let mut run = source_sync_run_record();
        run.status = (*status).to_string();
        run.mode = if *status == SYNC_RUN_STATUS_PLANNED {
            SYNC_RUN_MODE_DRY_RUN.to_string()
        } else {
            SYNC_RUN_MODE_APPLY.to_string()
        };
        package.source_sync_runs = vec![run];

        validate_package_manifest(&package)
            .expect("service-generated source sync run state should import");
    }
}

#[test]
fn validate_package_manifest_rejects_non_object_source_sync_run_summary() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.summary_json = json!(["not", "an", "object"]);
    package.source_sync_runs = vec![run];

    let err = validate_package_manifest(&package)
        .expect_err("source sync run summary_json must keep service object invariant");

    assert!(err.contains("源同步运行摘要必须是 JSON 对象"));
    assert!(err.contains("source_sync_run_id=801"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_conflict_run_context_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    package.source_sync_runs = vec![source_sync_run_record()];
    let mut conflict = source_sync_conflict_record();
    conflict.from_snapshot_id = 903;
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflicts must share the run source snapshot context");

    assert!(err.contains("源同步冲突与所属运行上下文不一致"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("sync_run_id=801"));
    assert!(err.contains("from_snapshot_id=903"));
}

#[test]
fn validate_package_manifest_accepts_source_sync_conflict_matching_run_context() {
    let package = package_with_source_sync_conflict(source_sync_conflict_record());

    validate_package_manifest(&package)
        .expect("source sync conflict with matching run context should remain importable");
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_conflict_target_combo() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.target_kind = "image".to_string();
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict target combos must match service-generated shapes");

    assert!(err.contains("源同步冲突目标类型组合无效"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("conflict_scope=generation_spec"));
    assert!(err.contains("target_kind=image"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_conflict_target_combos() {
    for (scope, target_kind) in [
        ("generation_spec", "segment"),
        ("text_anchor", "chapter"),
        ("text_anchor", "segment"),
        ("text_anchor", "galgame_scene"),
        ("text_anchor", "galgame_line"),
        ("profile", "character"),
        ("profile", "object"),
        ("profile", "scene"),
    ] {
        let mut conflict = source_sync_conflict_record();
        conflict.conflict_scope = scope.to_string();
        conflict.target_kind = target_kind.to_string();
        let package = package_with_source_sync_conflict(conflict);

        validate_package_manifest(&package)
            .expect("service-generated source sync conflict target combo should import");
    }
}

#[test]
fn validate_package_manifest_rejects_missing_source_sync_conflict_target() {
    let mut package = package_with_part_outlines(Vec::new());
    package.source_sync_runs = vec![source_sync_run_record()];
    let mut conflict = source_sync_conflict_record();
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_line".to_string();
    conflict.target_id = 999;
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict target_id must exist in the package");

    assert!(err.contains("源同步冲突目标不存在"));
    assert!(err.contains("target_kind=galgame_line"));
    assert!(err.contains("target_id=999"));
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_conflict_change_kind() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.source_change_kind = "renamed".to_string();
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict change kind must use known values");

    assert!(err.contains("源同步冲突源变更类型无效"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("source_change_kind=renamed"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_conflict_change_kinds() {
    for source_change_kind in SOURCE_CHANGE_KINDS {
        let mut conflict = source_sync_conflict_record();
        conflict.source_change_kind = (*source_change_kind).to_string();
        let package = package_with_source_sync_conflict(conflict);

        validate_package_manifest(&package)
            .expect("known source sync change kind should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_conflict_local_state() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.local_state_kind = "archived".to_string();
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict local state must use known values");

    assert!(err.contains("源同步冲突本地状态类型无效"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("local_state_kind=archived"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_conflict_local_states() {
    for local_state_kind in LOCAL_STATE_KINDS {
        let mut conflict = source_sync_conflict_record();
        conflict.local_state_kind = (*local_state_kind).to_string();
        let package = package_with_source_sync_conflict(conflict);

        validate_package_manifest(&package)
            .expect("known source sync local state should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_conflict_severity() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.severity = "major".to_string();
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict severity must use known values");

    assert!(err.contains("源同步冲突严重级别无效"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("severity=major"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_conflict_severities() {
    for severity in CONFLICT_SEVERITIES {
        let mut conflict = source_sync_conflict_record();
        conflict.severity = (*severity).to_string();
        let package = package_with_source_sync_conflict(conflict);

        validate_package_manifest(&package)
            .expect("known source sync severity should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_conflict_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.status = "closed".to_string();
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict status must use known values");

    assert!(err.contains("源同步冲突状态无效"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("status=closed"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_conflict_statuses() {
    for status in CONFLICT_STATUSES {
        let conflict = if *status == CONFLICT_STATUS_PENDING {
            source_sync_conflict_record()
        } else {
            handled_source_sync_conflict(status)
        };
        let package = package_with_source_sync_conflict(conflict);

        validate_package_manifest(&package)
            .expect("known source sync conflict status should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_conflict_resolution_strategy() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = handled_source_sync_conflict(CONFLICT_STATUS_RESOLVED);
    conflict.resolution_strategy = Some("archive".to_string());
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict resolution strategy must use known values");

    assert!(err.contains("源同步冲突解决策略无效"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("resolution_strategy=archive"));
}

#[test]
fn validate_package_manifest_rejects_pending_source_sync_conflict_with_resolution() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.resolution_strategy = Some("keep_local".to_string());
    conflict.decision_json = Some(json!({"reason": "already decided"}));
    conflict.resolved_at = Some(dt());
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("pending source sync conflicts must not include resolution results");

    assert!(err.contains("待处理源同步冲突不能包含解决结果"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_handled_source_sync_conflict_without_resolution() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.status = CONFLICT_STATUS_RESOLVED.to_string();
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("handled source sync conflicts must include resolution results");

    assert!(err.contains("已处理源同步冲突缺少解决结果"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_non_object_source_sync_conflict_summaries() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut conflict = source_sync_conflict_record();
    conflict.affected_refs_json = json!(["not", "an", "object"]);
    package.source_sync_conflicts = vec![conflict];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict JSON summaries must keep service object invariant");

    assert!(err.contains("源同步冲突影响引用必须是 JSON 对象"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_accepts_source_sync_conflict_json_refs() {
    let snapshot_chapter = source_snapshot_chapter_record(1);
    let mut conflict = source_sync_conflict_record();
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_line".to_string();
    conflict.target_id = 2301;
    conflict.source_summary_json = json!({
        "old_text_anchor_id": 702,
        "new_text_anchor_id": null
    });
    conflict.local_summary_json = json!({
        "galgame_line_id": 2301,
        "galgame_scene_id": 2201
    });
    conflict.affected_refs_json = json!({
        "old_text_anchor_id": 702,
        "new_text_anchor_id": null,
        "old_source_snapshot_chapter_id": 1001,
        "source_snapshot_chapter_id": null,
        "galgame_line_id": 2301,
        "galgame_scene_id": 2201
    });
    let mut package = package_with_source_sync_conflict(conflict);
    package.text_anchors = vec![snapshot_text_anchor_record(&snapshot_chapter)];

    validate_package_manifest(&package)
        .expect("service-generated source sync conflict JSON refs should remain importable");
}

#[test]
fn validate_package_manifest_rejects_missing_source_sync_conflict_json_ref() {
    let mut conflict = source_sync_conflict_record();
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_line".to_string();
    conflict.target_id = 2301;
    conflict.affected_refs_json = json!({
        "galgame_line_id": 2301,
        "galgame_scene_id": 999
    });
    let package = package_with_source_sync_conflict(conflict);

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict JSON refs must point to package rows");

    assert!(err.contains("affected_refs_json.galgame_scene_id"));
    assert!(err.contains("Galgame 场不存在"));
    assert!(err.contains("galgame_scene_id=999"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_conflict_line_scene_mismatch() {
    let mut conflict = source_sync_conflict_record();
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_line".to_string();
    conflict.target_id = 2301;
    conflict.local_summary_json = json!({
        "galgame_line_id": 2301,
        "galgame_scene_id": 2202
    });
    let mut package = package_with_source_sync_conflict(conflict);
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "main", None));

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict Galgame line refs must belong to the declared scene");

    assert!(err.contains("local_summary_json.galgame_line_id 不属于 galgame_scene_id"));
    assert!(err.contains("galgame_line_id=2301"));
    assert!(err.contains("expected=2202"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_conflict_anchor_chapter_mismatch() {
    let snapshot_chapter = source_snapshot_chapter_record(1);
    let mut conflict = source_sync_conflict_record();
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_line".to_string();
    conflict.target_id = 2301;
    conflict.affected_refs_json = json!({
        "old_text_anchor_id": 702,
        "old_source_snapshot_chapter_id": 1002,
        "galgame_line_id": 2301,
        "galgame_scene_id": 2201
    });
    let mut package = package_with_source_sync_conflict(conflict);
    package.text_anchors = vec![snapshot_text_anchor_record(&snapshot_chapter)];

    let err = validate_package_manifest(&package).expect_err(
        "source sync conflict anchor refs must belong to their declared snapshot chapter",
    );

    assert!(
        err.contains("affected_refs_json.old_text_anchor_id 不属于 old_source_snapshot_chapter_id")
    );
    assert!(err.contains("old_text_anchor_id=702"));
    assert!(err.contains("old_source_snapshot_chapter_id=1002"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_conflict_snapshot_source_chapter_mismatch() {
    let mut conflict = source_sync_conflict_record();
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_scene".to_string();
    conflict.target_id = 2201;
    conflict.affected_refs_json = json!({
        "source_chapter_id": 2002,
        "source_snapshot_chapter_id": 1001,
        "galgame_scene_id": 2201
    });
    let mut package = package_with_source_sync_conflict(conflict);
    package.text_core.chapters = vec![chapter_record(2001, 1, None), chapter_record(2002, 2, None)];

    let err = validate_package_manifest(&package)
        .expect_err("source sync conflict source chapter must match snapshot chapter");

    assert!(
        err.contains("affected_refs_json.source_chapter_id 与 source_snapshot_chapter_id 不一致")
    );
    assert!(err.contains("source_chapter_id=2002"));
    assert!(err.contains("expected_source_chapter_id=2001"));
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_decision_actor() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut decision = source_sync_decision_record("keep_local");
    decision.decided_by = "robot".to_string();
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision actor must use service-generated values");

    assert!(err.contains("源同步决策人无效"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("decided_by=robot"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_decision_actors() {
    for actor in DECISION_ACTORS {
        let mut decision = source_sync_decision_record("keep_local");
        decision.decided_by = (*actor).to_string();
        let mut package =
            package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
                CONFLICT_STATUS_RESOLVED,
                "keep_local",
                decision.decision_payload_json.clone(),
            ));
        package.source_sync_decisions = vec![decision];

        validate_package_manifest(&package)
            .expect("known source sync decision actor should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_decision_strategy() {
    let mut package = package_with_part_outlines(Vec::new());
    package.source_sync_decisions = vec![source_sync_decision_record("archive")];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision strategy must use known values");

    assert!(err.contains("源同步决策策略无效"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("strategy=archive"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_decision_strategies() {
    for strategy in RESOLUTION_STRATEGIES {
        let decision_payload_json = if *strategy == "regenerate_scene" {
            regenerate_scene_decision_payload()
        } else {
            json!({})
        };
        let mut decision = source_sync_decision_record(strategy);
        decision.decision_payload_json = decision_payload_json.clone();
        let mut package = if *strategy == "regenerate_scene" {
            package_with_regenerate_scene_decision(decision_payload_json)
        } else {
            package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
                CONFLICT_STATUS_RESOLVED,
                strategy,
                decision_payload_json,
            ))
        };
        package.source_sync_decisions = vec![decision];

        validate_package_manifest(&package)
            .expect("known source sync decision strategy should remain importable");
    }
}

#[test]
fn validate_package_manifest_rejects_non_object_source_sync_decision_payload() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut decision = source_sync_decision_record("keep_local");
    decision.decision_payload_json = json!(["not", "an", "object"]);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision payload must keep service object invariant");

    assert!(err.contains("源同步决策载荷必须是 JSON 对象"));
    assert!(err.contains("source_sync_decision_id=821"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_decision_for_pending_conflict() {
    let mut package = package_with_source_sync_conflict(source_sync_conflict_record());
    package.source_sync_decisions = vec![source_sync_decision_record("keep_local")];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decisions require the referenced conflict to be handled");

    assert!(err.contains("源同步决策引用了待处理源同步冲突"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_decision_strategy_mismatch() {
    let decision = source_sync_decision_record("manual");
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        decision.decision_payload_json.clone(),
    ));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision strategy must match conflict resolution_strategy");

    assert!(err.contains("源同步决策策略与冲突解决策略不一致"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("decision_strategy=manual"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_decision_payload_mismatch() {
    let decision = source_sync_decision_record("keep_local");
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        json!({"reason": "different"}),
    ));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source sync decision payload must match conflict decision_json");

    assert!(err.contains("源同步决策载荷与冲突决策载荷不一致"));
    assert!(err.contains("source_sync_decision_id=821"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_accepts_source_sync_decision_matching_conflict() {
    let decision = source_sync_decision_record("keep_local");
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        decision.decision_payload_json.clone(),
    ));
    package.source_sync_decisions = vec![decision];

    validate_package_manifest(&package)
        .expect("source sync decision matching its handled conflict should import");
}

#[test]
fn validate_package_manifest_accepts_regenerate_scene_stage_outputs() {
    let payload = regenerate_scene_decision_payload();
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    validate_package_manifest(&package)
        .expect("regenerate_scene stage output references should remain importable");
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_result() {
    let mut payload = regenerate_scene_decision_payload();
    payload
        .as_object_mut()
        .expect("fixture payload must be object")
        .remove("result");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("handled regenerate_scene decisions must keep runtime result");

    assert!(err.contains("regenerate_scene result 缺失"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_null_result() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"] = Value::Null;
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("handled regenerate_scene result must not be null");

    assert!(err.contains("regenerate_scene result 必须是 JSON 对象"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_result_scene_id() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("galgame_scene_id");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep rebuilt scene id");

    assert!(err.contains("regenerate_scene result.galgame_scene_id 必须是整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_old_snapshot_chapter_id() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("old_source_snapshot_chapter_id");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep old source snapshot chapter id");

    assert!(err.contains("regenerate_scene result.old_source_snapshot_chapter_id 必须是整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_new_snapshot_chapter_id() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("new_source_snapshot_chapter_id");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep new source snapshot chapter id");

    assert!(err.contains("regenerate_scene result.new_source_snapshot_chapter_id 必须是整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_stage_outputs() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("stage_outputs");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must keep runtime stage outputs");

    assert!(err.contains("regenerate_scene result.stage_outputs 缺失"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_null_stage_outputs() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"] = Value::Null;
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result stage_outputs must not be null");

    assert!(err.contains("regenerate_scene result.stage_outputs 必须是 JSON 对象"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_result_count() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]
        .as_object_mut()
        .expect("fixture result must be object")
        .remove("deleted_portrait_generation_spec_count");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result cleanup counters must be complete");

    assert!(err.contains("result.deleted_portrait_generation_spec_count 缺失"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_result_count() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["deleted_portrait_generation_spec_count"] = json!(-1);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result cleanup counters must be non-negative integers");

    assert!(err.contains("result.deleted_portrait_generation_spec_count 必须是非负整数"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_unknown_payload_field() {
    let mut payload = regenerate_scene_decision_payload();
    payload["silent_fallback"] = json!(true);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload must reject unknown fields");

    assert!(err.contains("regenerate_scene 决策载荷.silent_fallback 是未知字段"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_user_action() {
    let mut payload = regenerate_scene_decision_payload();
    payload
        .as_object_mut()
        .expect("fixture payload must be object")
        .remove("user_action");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload must keep explicit user_action");

    assert!(err.contains("regenerate_scene 决策载荷.user_action 必须是字符串"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_user_action() {
    let mut payload = regenerate_scene_decision_payload();
    payload["user_action"] = json!("keep_local");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload user_action must match the strategy");

    assert!(err.contains("regenerate_scene 决策载荷.user_action 无效"));
    assert!(err.contains("user_action=keep_local"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_reason_type() {
    let mut payload = regenerate_scene_decision_payload();
    payload["reason"] = json!({"kind": "source_changed"});
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload reason must keep service JSON shape");

    assert!(err.contains("regenerate_scene 决策载荷.reason 必须是字符串或 null"));
    assert!(err.contains("source_sync_conflict_id=811"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_invalid_result_mode() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["mode"] = json!("baseline_fallback");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result mode must remain explicit LLM partial regeneration");

    assert!(err.contains("regenerate_scene result.mode 无效"));
    assert!(err.contains("baseline_fallback"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_unknown_result_field() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["legacy_summary"] = json!("fallback rebuilt from anchors");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result must reject unknown fields");

    assert!(err.contains("regenerate_scene result.legacy_summary 是未知字段"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_missing_stage_output() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]
        .as_object_mut()
        .expect("fixture stage_outputs must be object")
        .remove("visual_planning");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs must contain all runtime stages");

    assert!(err.contains("result.stage_outputs.visual_planning 缺失"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_unknown_stage_output() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["legacy_anchor_rebuild"] = json!({});
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs must reject unknown stages");

    assert!(err.contains("regenerate_scene result.stage_outputs.legacy_anchor_rebuild 是未知字段"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_stage_output_missing_speaker() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["character_staging"]["scene_lines"][0]["lines"][0]["speaker_profile_id"] =
        json!(999);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage output speaker_profile_id must exist in package");

    assert!(err.contains("speaker_profile_id"));
    assert!(err.contains("角色档案不存在"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_result_scene_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["galgame_scene_id"] = json!(2202);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "main", None));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result scene must match the decision target scene");

    assert!(err.contains("result.galgame_scene_id 与决策目标场不一致"));
    assert!(err.contains("galgame_scene_id=2202"));
    assert!(err.contains("expected=2201"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_payload_target_scene_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["galgame_scene_id"] = json!(2202);
    payload["result"]["galgame_scene_id"] = json!(2202);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package
        .galgame_scenes
        .push(galgame_scene_record_with_id(2202, 1001, "main", None));
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene payload scene must match the conflict target scene");

    assert!(err.contains("regenerate_scene galgame_scene_id 与冲突目标场不一致"));
    assert!(err.contains("source_sync_conflict_id=811"));
    assert!(err.contains("galgame_scene_id=2202"));
    assert!(err.contains("expected=2201"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_result_new_chapter_scene_mismatch() {
    let payload = regenerate_scene_decision_payload();
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.galgame_scenes[0].source_snapshot_chapter_id = 1002;
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene result chapter must match the current Galgame scene chapter");

    assert!(
        err.contains("result.new_source_snapshot_chapter_id 与 Galgame 场当前源快照章节不一致")
    );
    assert!(err.contains("galgame_scene_id=2201"));
    assert!(err.contains("scene_source_snapshot_chapter_id=1002"));
    assert!(err.contains("result_new_source_snapshot_chapter_id=1001"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_stage_output_chapter_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["scene_segmentation"]["scenes"][0]["source_snapshot_chapter_id"] =
        json!(1002);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("stage output source_snapshot_chapter_id must match regenerated chapter");

    assert!(err.contains("source_snapshot_chapter_id 与重建目标章节不一致"));
    assert!(err.contains("source_snapshot_chapter_id=1002"));
    assert!(err.contains("expected=1001"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_source_span_chapter_mismatch() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["scene_segmentation"]["scenes"][0]["source_span_ids"] =
        json!(["ch2:ta702"]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source_span_ids chapter number must match the referenced text anchor");

    assert!(err.contains("source_span_ids 章节编号与重建目标章节不一致"));
    assert!(err.contains("source_span_id=ch2:ta702"));
    assert!(err.contains("expected=1"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_source_span_outside_target_chapter() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["scene_segmentation"]["scenes"][0]["source_span_ids"] =
        json!(["ch2:ta703"]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    let mut second_anchor = snapshot_text_anchor_record(&package.source_snapshot_chapters[1]);
    second_anchor.id = 703;
    package.text_anchors.push(second_anchor);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("source_span_ids anchors must belong to the regenerated snapshot chapter");

    assert!(err.contains("source_span_ids 章节编号与重建目标章节不一致"));
    assert!(err.contains("source_span_id=ch2:ta703"));
    assert!(err.contains("expected=1"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_scene_segmentation_empty_source_span() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["scene_segmentation"]["scenes"][0]["source_span_ids"] =
        json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene scene_segmentation scenes must keep source evidence");

    assert!(err.contains("result.stage_outputs.scene_segmentation.scenes[0].source_span_ids"));
    assert!(err.contains("必须是非空字符串数组"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_missing_scene_lines() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]
        .as_object_mut()
        .expect("fixture script_adaptation must be object")
        .remove("scene_lines");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation must contain scene_lines");

    assert!(err.contains("result.stage_outputs.script_adaptation.scene_lines 缺失"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_unknown_field() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]["legacy_lines"] = json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation must reject unknown stage fields");

    assert!(err.contains("result.stage_outputs.script_adaptation.legacy_lines 是未知字段"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_multiple_scenes() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]["scene_lines"]
        .as_array_mut()
        .expect("fixture script_adaptation scene_lines must be array")
        .push(json!({
            "scene_index": 2,
            "lines": [{
                "index": 1,
                "line_type": "narration",
                "text": "第二场不允许出现在局部重跑结果。",
                "source_span_ids": ["ch1:ta702"]
            }]
        }));
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation must remain single-scene");

    assert!(err.contains("result.stage_outputs.script_adaptation.scene_lines"));
    assert!(err.contains("必须且只能包含一个 scene_index=1 场"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_script_adaptation_line_without_source_span() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["script_adaptation"]["scene_lines"][0]["lines"][0]["source_span_ids"] =
        json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene script_adaptation lines must keep source evidence");

    assert!(err.contains(
        "result.stage_outputs.script_adaptation.scene_lines[0].lines[0].source_span_ids"
    ));
    assert!(err.contains("必须是非空字符串数组"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_character_staging_line_missing_source_span() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["character_staging"]["scene_lines"][0]["lines"][0]
        .as_object_mut()
        .expect("fixture staged line must be object")
        .remove("source_span_ids");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene character_staging lines must keep source evidence");

    assert!(err.contains(
        "result.stage_outputs.character_staging.scene_lines[0].lines[0].source_span_ids"
    ));
    assert!(err.contains("必须是非空字符串数组"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_event_cg_wrong_scene() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]["event_cgs"] = json!([{
        "scene_index": 2,
        "line_index": 1,
        "cg_note": "跨场 CG 不允许出现在局部重跑结果。",
        "importance": "normal",
        "source_span_ids": ["ch1:ta702"]
    }]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning event CG must reference scene_index=1");

    assert!(err.contains("result.stage_outputs.visual_planning.event_cgs[0].scene_index 必须为 1"));
    assert!(err.contains("scene_index=2"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_background_empty_source_span()
{
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]["backgrounds"][0]["source_span_ids"] =
        json!([]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning backgrounds must keep source evidence");

    assert!(err.contains("result.stage_outputs.visual_planning.backgrounds[0].source_span_ids"));
    assert!(err.contains("必须是非空字符串数组"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_missing_backgrounds() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]
        .as_object_mut()
        .expect("fixture visual_planning must be object")
        .remove("backgrounds");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning must contain backgrounds");

    assert!(err.contains("result.stage_outputs.visual_planning.backgrounds 缺失"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_missing_event_cgs() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]
        .as_object_mut()
        .expect("fixture visual_planning must be object")
        .remove("event_cgs");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning must contain event_cgs");

    assert!(err.contains("result.stage_outputs.visual_planning.event_cgs 缺失"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_visual_planning_event_cg_missing_source_span()
{
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["visual_planning"]["event_cgs"] = json!([{
        "scene_index": 1,
        "line_index": 1,
        "cg_note": "缺少 source_span_ids 的 CG 不允许导入。",
        "importance": "normal"
    }]);
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene visual_planning event CGs must keep source evidence");

    assert!(err.contains("result.stage_outputs.visual_planning.event_cgs[0].source_span_ids"));
    assert!(err.contains("必须是非空字符串数组"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_branch_design_missing_choices() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["branch_design"]
        .as_object_mut()
        .expect("fixture branch_design must be object")
        .remove("choices");
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene branch_design must contain choices");

    assert!(err.contains("result.stage_outputs.branch_design.choices 缺失"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_branch_design_choices() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["branch_design"] = json!({
        "choices": [{
            "scene_index": 1,
            "after_line_index": 1,
            "question": "要不要追上去？",
            "options": [
                {
                    "option_text": "追上去",
                    "branch_title": "追逐",
                    "branch_summary": "主角追上对方。",
                    "returns_to_scene_index": 1
                },
                {
                    "option_text": "留下",
                    "branch_title": "停留",
                    "branch_summary": "主角选择留在原地。",
                    "returns_to_scene_index": 1
                }
            ]
        }]
    });
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs branch_design must not add choices");

    assert!(err.contains("result.stage_outputs.branch_design.choices 必须为空"));
    assert!(err.contains("局部重跑不允许新增选择支"));
}

#[test]
fn validate_package_manifest_rejects_regenerate_scene_branch_design_non_array_choices() {
    let mut payload = regenerate_scene_decision_payload();
    payload["result"]["stage_outputs"]["branch_design"] = json!({
        "choices": {}
    });
    let mut decision = source_sync_decision_record("regenerate_scene");
    decision.decision_payload_json = payload.clone();
    let mut package = package_with_regenerate_scene_decision(payload);
    package.source_sync_decisions = vec![decision];

    let err = validate_package_manifest(&package)
        .expect_err("regenerate_scene stage_outputs branch_design choices must be an array");

    assert!(err.contains("result.stage_outputs.branch_design.choices 必须是数组"));
}

#[test]
fn validate_package_manifest_rejects_duplicate_source_sync_decision_for_conflict() {
    let decision = source_sync_decision_record("keep_local");
    let mut duplicate = source_sync_decision_record("keep_local");
    duplicate.id = 822;
    let mut package = package_with_source_sync_conflict(handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "keep_local",
        decision.decision_payload_json.clone(),
    ));
    package.source_sync_decisions = vec![decision, duplicate];

    let err = validate_package_manifest(&package)
        .expect_err("a handled source sync conflict can only have one decision record");

    assert!(err.contains("重复 source_sync_decisions.conflict_id"));
    assert!(err.contains("811"));
}

fn package_with_regenerate_scene_decision(decision_payload_json: Value) -> ProjectFamilyPackage {
    let mut package = package_with_galgame_project();
    let snapshot_chapter = package.source_snapshot_chapters[0].clone();
    package.text_core.chapters = vec![chapter_record(snapshot_chapter.source_chapter_id, 1, None)];
    package.text_core.character_profiles = vec![character_profile_record(601)];
    package.text_core.scene_profiles = vec![scene_profile_record(701)];
    package.text_anchors = vec![snapshot_text_anchor_record(&snapshot_chapter)];
    package.source_sync_runs = vec![source_sync_run_record()];
    let mut conflict = handled_source_sync_conflict_for_decision(
        CONFLICT_STATUS_RESOLVED,
        "regenerate_scene",
        decision_payload_json,
    );
    conflict.conflict_scope = "text_anchor".to_string();
    conflict.target_kind = "galgame_scene".to_string();
    conflict.target_id = 2201;
    package.source_sync_conflicts = vec![conflict];
    package
}

fn regenerate_scene_decision_payload() -> Value {
    json!({
        "user_action": "regenerate_scene",
        "galgame_scene_id": 2201,
        "reason": "source_changed",
        "result": {
            "galgame_scene_id": 2201,
            "old_source_snapshot_chapter_id": 1001,
            "new_source_snapshot_chapter_id": 1001,
            "old_line_count": 1,
            "new_line_count": 1,
            "deleted_generation_spec_count": 0,
            "deleted_portrait_generation_spec_count": 0,
            "resolved_related_conflict_count": 0,
            "mode": "llm_partial_regeneration",
            "stage_outputs": {
                "scene_segmentation": {
                    "scenes": [
                        {
                            "index": 1,
                            "source_snapshot_chapter_id": 1001,
                            "source_chapter_id": 2001,
                            "source_span_ids": ["ch1:ta702"]
                        }
                    ]
                },
                "script_adaptation": {
                    "scene_lines": [
                        {
                            "scene_index": 1,
                            "lines": [
                                {
                                    "index": 1,
                                    "line_type": "narration",
                                    "text": "雨停了。",
                                    "source_span_ids": ["ch1:ta702"]
                                }
                            ]
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
                                    "speaker_profile_id": 601,
                                    "galgame_line_id": 2301,
                                    "text": "雨停了。",
                                    "source_span_ids": ["ch1:ta702"]
                                }
                            ]
                        }
                    ]
                },
                "visual_planning": {
                    "backgrounds": [
                        {
                            "scene_index": 1,
                            "background_note": "雨后的街道。",
                            "scene_profile_id": 701,
                            "galgame_scene_id": 2201,
                            "source_span_ids": ["ch1:ta702"]
                        }
                    ],
                    "event_cgs": []
                },
                "branch_design": {
                    "choices": []
                }
            }
        }
    })
}
