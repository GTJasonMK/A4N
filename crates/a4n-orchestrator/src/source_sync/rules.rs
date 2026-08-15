use super::contracts::SourceSyncDecisionInput;
use a4n_domain::source_sync::SourceSyncResolutionStrategy;

pub(super) fn source_sync_decisions_require_galgame_preparation(
    decisions: &[SourceSyncDecisionInput],
) -> bool {
    decisions
        .iter()
        .any(|decision| decision.strategy == SourceSyncResolutionStrategy::RegenerateScene)
}
