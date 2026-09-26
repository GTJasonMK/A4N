use std::collections::HashMap;

use a4n_db_models::models::source_sync::source_sync_conflict;
use a4n_domain::source_sync::{
    SourceSyncConflictStatus, SourceSyncDecisionActor, SourceSyncResolutionStrategy,
};

use super::super::super::ProjectFamilyPackage;
use super::super::contracts::SourceSyncReferenceSets;
use super::super::decision::validate_source_sync_decision_payload;
use super::super::rules::validate_source_sync_decision_conflict;

pub(super) fn validate_source_sync_decisions(
    package: &ProjectFamilyPackage,
    refs: &SourceSyncReferenceSets,
    conflicts: &HashMap<i64, &source_sync_conflict::Model>,
) -> Result<(), String> {
    for decision in &package.source_sync_decisions {
        ensure_package!(
            SourceSyncDecisionActor::parse(&decision.decided_by).is_some(),
            "项目族导入包源同步决策人无效: source_sync_decision_id={}, decided_by={}",
            decision.id,
            decision.decided_by
        );
        ensure_package!(
            SourceSyncResolutionStrategy::parse(&decision.strategy).is_some(),
            "项目族导入包源同步决策策略无效: source_sync_decision_id={}, strategy={}",
            decision.id,
            decision.strategy
        );
        ensure_package!(
            decision.decision_payload_json.is_object(),
            "项目族导入包源同步决策载荷必须是 JSON 对象: source_sync_decision_id={}",
            decision.id
        );
        validate_source_sync_decision_conflict(decision, conflicts)?;
        if conflicts[&decision.conflict_id].status == SourceSyncConflictStatus::Obsolete.as_str() {
            continue;
        }
        validate_source_sync_decision_payload(
            "source_sync_decision",
            decision.id,
            &decision.strategy,
            &decision.decision_payload_json,
            refs,
        )?;
    }
    Ok(())
}
