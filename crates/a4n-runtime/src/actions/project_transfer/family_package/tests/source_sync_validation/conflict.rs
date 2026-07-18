use super::*;

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
