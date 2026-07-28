pub use a4n_api_contract::source_sync::{
    ApplySourceSyncInput, SourceSyncConflictSummary, SourceSyncDecisionInput,
    SourceSyncPlanSummary, SourceSyncRunSummary,
};

pub(super) fn orchestrator_decision(
    decision: SourceSyncDecisionInput,
) -> a4n_orchestrator::source_sync::SourceSyncDecisionInput {
    a4n_orchestrator::source_sync::SourceSyncDecisionInput {
        conflict_id: decision.conflict_id,
        strategy: decision.strategy,
        decision_payload_json: decision.decision_payload_json,
    }
}
