use super::*;
use a4n_domain::source_sync::SourceSyncConflictStatus;

pub(super) fn is_valid_source_sync_conflict_target_ref(
    conflict_scope: &str,
    target_kind: &str,
) -> bool {
    matches!(
        (conflict_scope, target_kind),
        ("generation_spec", "segment")
            | ("text_anchor", "chapter")
            | ("text_anchor", "segment")
            | ("text_anchor", "galgame_scene")
            | ("text_anchor", "galgame_line")
            | ("profile", "character")
            | ("profile", "object")
            | ("profile", "scene")
    )
}

pub(super) fn validate_source_sync_decision_conflict(
    decision: &a4n_domain::project::ProjectFamilySourceSyncDecisionRecord,
    conflicts: &HashMap<i64, &a4n_domain::project::ProjectFamilySourceSyncConflictRecord>,
) -> Result<(), String> {
    let conflict = conflicts.get(&decision.conflict_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包源同步决策引用了不存在的源同步冲突: source_sync_decision_id={}, conflict_id={}",
            decision.id, decision.conflict_id
        )
    })?;
    if conflict.status == SourceSyncConflictStatus::Pending.as_str() {
        return Err(format!(
            "项目族导入包源同步决策引用了待处理源同步冲突: source_sync_decision_id={}, source_sync_conflict_id={}",
            decision.id, conflict.id
        ));
    }
    if conflict.resolution_strategy.as_deref() != Some(decision.strategy.as_str()) {
        return Err(format!(
            "项目族导入包源同步决策策略与冲突解决策略不一致: source_sync_decision_id={}, source_sync_conflict_id={}, decision_strategy={}, conflict_resolution_strategy={:?}",
            decision.id, conflict.id, decision.strategy, conflict.resolution_strategy
        ));
    }
    if conflict.decision_json.as_ref() != Some(&decision.decision_payload_json) {
        return Err(format!(
            "项目族导入包源同步决策载荷与冲突决策载荷不一致: source_sync_decision_id={}, source_sync_conflict_id={}",
            decision.id, conflict.id
        ));
    }
    Ok(())
}

pub(super) fn validate_source_sync_conflict_run_context(
    conflict: &a4n_domain::project::ProjectFamilySourceSyncConflictRecord,
    run: &a4n_domain::project::ProjectFamilySourceSyncRunRecord,
) -> Result<(), String> {
    if conflict.derived_project_id != run.derived_project_id
        || conflict.root_project_id != run.root_project_id
        || conflict.from_snapshot_id != run.from_snapshot_id
        || conflict.to_snapshot_id != run.to_snapshot_id
    {
        return Err(format!(
            "项目族导入包源同步冲突与所属运行上下文不一致: source_sync_conflict_id={}, sync_run_id={}, conflict=(derived_project_id={}, root_project_id={}, from_snapshot_id={}, to_snapshot_id={}), run=(derived_project_id={}, root_project_id={}, from_snapshot_id={}, to_snapshot_id={})",
            conflict.id,
            conflict.sync_run_id,
            conflict.derived_project_id,
            conflict.root_project_id,
            conflict.from_snapshot_id,
            conflict.to_snapshot_id,
            run.derived_project_id,
            run.root_project_id,
            run.from_snapshot_id,
            run.to_snapshot_id
        ));
    }
    Ok(())
}
