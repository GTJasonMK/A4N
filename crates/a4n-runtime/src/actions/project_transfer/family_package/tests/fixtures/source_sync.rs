use super::super::*;

pub(in crate::actions::project_transfer::family_package::tests) fn source_sync_run_record()
-> source_sync_run::Model {
    source_sync_run::Model {
        id: 801,
        derived_project_id: 2,
        root_project_id: 1,
        from_snapshot_id: 901,
        to_snapshot_id: 902,
        mode: SYNC_RUN_MODE_DRY_RUN.to_string(),
        status: SYNC_RUN_STATUS_PLANNED.to_string(),
        summary_json: json!({}),
        error_message: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::actions::project_transfer::family_package::tests) fn source_sync_conflict_record()
-> source_sync_conflict::Model {
    source_sync_conflict::Model {
        id: 811,
        sync_run_id: 801,
        derived_project_id: 2,
        root_project_id: 1,
        from_snapshot_id: 901,
        to_snapshot_id: 902,
        conflict_scope: "generation_spec".to_string(),
        target_kind: "segment".to_string(),
        target_id: 301,
        source_change_kind: "updated".to_string(),
        local_state_kind: "draft".to_string(),
        severity: "warning".to_string(),
        status: CONFLICT_STATUS_PENDING.to_string(),
        resolution_strategy: None,
        source_summary_json: json!({}),
        local_summary_json: json!({}),
        affected_refs_json: json!({}),
        decision_json: None,
        created_at: dt(),
        resolved_at: None,
    }
}

pub(in crate::actions::project_transfer::family_package::tests) fn handled_source_sync_conflict(
    status: &str,
) -> source_sync_conflict::Model {
    let mut conflict = source_sync_conflict_record();
    conflict.status = status.to_string();
    conflict.resolution_strategy = Some("keep_local".to_string());
    conflict.decision_json = Some(json!({"reason": "kept"}));
    conflict.resolved_at = Some(dt());
    conflict
}

pub(in crate::actions::project_transfer::family_package::tests) fn handled_source_sync_conflict_for_decision(
    status: &str,
    strategy: &str,
    decision_payload_json: Value,
) -> source_sync_conflict::Model {
    let mut conflict = source_sync_conflict_record();
    conflict.status = status.to_string();
    conflict.resolution_strategy = Some(strategy.to_string());
    conflict.decision_json = Some(decision_payload_json);
    conflict.resolved_at = Some(dt());
    conflict
}

pub(in crate::actions::project_transfer::family_package::tests) fn source_sync_decision_record(
    strategy: &str,
) -> source_sync_decision::Model {
    source_sync_decision::Model {
        id: 821,
        conflict_id: 811,
        decided_by: "user".to_string(),
        strategy: strategy.to_string(),
        decision_payload_json: json!({}),
        created_at: dt(),
    }
}

pub(in crate::actions::project_transfer::family_package::tests) fn package_with_source_sync_conflict(
    conflict: source_sync_conflict::Model,
) -> ProjectFamilyPackage {
    let mut package = package_with_source_sync_conflict_target(&conflict);
    package.source_sync_runs = vec![source_sync_run_record()];
    package.source_sync_conflicts = vec![conflict];
    package
}

pub(in crate::actions::project_transfer::family_package::tests) fn package_with_source_sync_conflict_target(
    conflict: &source_sync_conflict::Model,
) -> ProjectFamilyPackage {
    match (
        conflict.conflict_scope.as_str(),
        conflict.target_kind.as_str(),
    ) {
        ("generation_spec", "segment") | ("text_anchor", "segment") => {
            let mut package = package_with_derived_project();
            package.generation_specs = vec![generation_spec_for_source_snapshot(
                source_sync_run_record().from_snapshot_id,
            )];
            package
        }
        ("text_anchor", "chapter") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.chapters = vec![chapter_record(conflict.target_id, 1, None)];
            package
        }
        ("text_anchor", "galgame_scene") => {
            let mut package = package_with_galgame_project();
            package.galgame_scenes = vec![galgame_scene_record_with_id(
                conflict.target_id,
                1001,
                "main",
                None,
            )];
            package.galgame_lines = vec![galgame_line_record(conflict.target_id)];
            package
        }
        ("text_anchor", "galgame_line") => {
            let mut package = package_with_galgame_project();
            package.galgame_scenes = vec![galgame_scene_record(1001)];
            package.galgame_lines = vec![galgame_line_record_with_id(conflict.target_id, 2201)];
            package
        }
        ("profile", "character") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.character_profiles =
                vec![character_profile_record(conflict.target_id)];
            package
        }
        ("profile", "object") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.object_profiles = vec![object_profile_record(conflict.target_id)];
            package
        }
        ("profile", "scene") => {
            let mut package = package_with_part_outlines(Vec::new());
            package.text_core.scene_profiles = vec![scene_profile_record(conflict.target_id)];
            package
        }
        _ => package_with_part_outlines(Vec::new()),
    }
}
