use std::collections::HashMap;

use a4n_core::models::source_sync::source_sync_run;
use a4n_orchestrator::source_sync::{
    CONFLICT_STATUS_IGNORED, CONFLICT_STATUS_PENDING, CONFLICT_STATUS_RESOLVED,
    is_valid_conflict_severity, is_valid_conflict_status, is_valid_local_state_kind,
    is_valid_resolution_strategy, is_valid_source_change_kind,
};

use super::super::super::ProjectFamilyPackage;
use super::super::contracts::SourceSyncReferenceSets;
use super::super::decision::validate_source_sync_decision_payload;
use super::super::references::{
    validate_source_sync_conflict_json_refs, validate_source_sync_conflict_regenerate_scene_target,
    validate_source_sync_conflict_target_ref,
};
use super::super::rules::{
    is_valid_source_sync_conflict_target_ref, validate_source_sync_conflict_run_context,
};

pub(super) fn validate_source_sync_conflicts(
    package: &ProjectFamilyPackage,
    refs: &SourceSyncReferenceSets,
    runs: &HashMap<i64, &source_sync_run::Model>,
) -> Result<(), String> {
    for conflict in &package.source_sync_conflicts {
        ensure_package!(
            is_valid_source_sync_conflict_target_ref(
                &conflict.conflict_scope,
                &conflict.target_kind,
            ),
            "项目族导入包源同步冲突目标类型组合无效: source_sync_conflict_id={}, conflict_scope={}, target_kind={}",
            conflict.id,
            conflict.conflict_scope,
            conflict.target_kind
        );
        ensure_package!(
            is_valid_source_change_kind(&conflict.source_change_kind),
            "项目族导入包源同步冲突源变更类型无效: source_sync_conflict_id={}, source_change_kind={}",
            conflict.id,
            conflict.source_change_kind
        );
        ensure_package!(
            is_valid_local_state_kind(&conflict.local_state_kind),
            "项目族导入包源同步冲突本地状态类型无效: source_sync_conflict_id={}, local_state_kind={}",
            conflict.id,
            conflict.local_state_kind
        );
        ensure_package!(
            is_valid_conflict_severity(&conflict.severity),
            "项目族导入包源同步冲突严重级别无效: source_sync_conflict_id={}, severity={}",
            conflict.id,
            conflict.severity
        );
        ensure_package!(
            is_valid_conflict_status(&conflict.status),
            "项目族导入包源同步冲突状态无效: source_sync_conflict_id={}, status={}",
            conflict.id,
            conflict.status
        );
        if let Some(strategy) = &conflict.resolution_strategy
            && !is_valid_resolution_strategy(strategy)
        {
            return Err(format!(
                "项目族导入包源同步冲突解决策略无效: source_sync_conflict_id={}, resolution_strategy={}",
                conflict.id, strategy
            ));
        }
        ensure_package!(
            conflict.source_summary_json.is_object(),
            "项目族导入包源同步冲突源摘要必须是 JSON 对象: source_sync_conflict_id={}",
            conflict.id
        );
        ensure_package!(
            conflict.local_summary_json.is_object(),
            "项目族导入包源同步冲突本地摘要必须是 JSON 对象: source_sync_conflict_id={}",
            conflict.id
        );
        ensure_package!(
            conflict.affected_refs_json.is_object(),
            "项目族导入包源同步冲突影响引用必须是 JSON 对象: source_sync_conflict_id={}",
            conflict.id
        );
        validate_source_sync_conflict_json_refs(conflict, refs)?;
        match conflict.status.as_str() {
            CONFLICT_STATUS_PENDING
                if conflict.resolution_strategy.is_some()
                    || conflict.decision_json.is_some()
                    || conflict.resolved_at.is_some() =>
            {
                return Err(format!(
                    "项目族导入包待处理源同步冲突不能包含解决结果: source_sync_conflict_id={}",
                    conflict.id
                ));
            }
            CONFLICT_STATUS_RESOLVED | CONFLICT_STATUS_IGNORED
                if conflict.resolution_strategy.is_none()
                    || conflict.decision_json.is_none()
                    || conflict.resolved_at.is_none() =>
            {
                return Err(format!(
                    "项目族导入包已处理源同步冲突缺少解决结果: source_sync_conflict_id={}",
                    conflict.id
                ));
            }
            _ => {}
        }
        let run = runs.get(&conflict.sync_run_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突引用了不存在的源同步运行: source_sync_conflict_id={}, sync_run_id={}",
                conflict.id, conflict.sync_run_id
            )
        })?;
        validate_source_sync_conflict_run_context(conflict, run)?;
        validate_source_sync_conflict_target_ref(conflict, refs)?;
        if let (Some(strategy), Some(payload)) = (
            conflict.resolution_strategy.as_deref(),
            conflict.decision_json.as_ref(),
        ) {
            validate_source_sync_conflict_regenerate_scene_target(
                conflict, strategy, payload, refs,
            )?;
            validate_source_sync_decision_payload(
                "source_sync_conflict",
                conflict.id,
                strategy,
                payload,
                refs,
            )?;
        }
    }
    Ok(())
}
