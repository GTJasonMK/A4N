pub use a4n_api_contract::optimize::{
    AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
    CreateOptimizationChatSessionInput, OptimizationAgentPermissions, OptimizationPatch,
    OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnEventSummary,
    OptimizationTurnSummary, PrepareOptimizationAnalysisSessionInput,
    ProposeOptimizationPatchesInput, SendOptimizationMessageInput,
};

pub(super) fn orchestrator_permissions(
    permissions: &OptimizationAgentPermissions,
) -> a4n_orchestrator::optimize::OptimizeAgentPermissions {
    a4n_orchestrator::optimize::OptimizeAgentPermissions {
        read_project_detail: permissions.read_project_detail,
        read_blueprint: permissions.read_blueprint,
        read_outlines: permissions.read_outlines,
        read_character_profiles: permissions.read_character_profiles,
        read_foreshadowing: permissions.read_foreshadowing,
        automatic_writeback: permissions.automatic_writeback,
    }
}

pub(super) fn orchestrator_patch(
    patch: &OptimizationPatch,
) -> a4n_orchestrator::optimize::OptimizationPatch {
    a4n_orchestrator::optimize::OptimizationPatch {
        patch_id: patch.patch_id.clone(),
        issue_id: patch.issue_id.clone(),
        operation: patch.operation.clone(),
        old_text: patch.old_text.clone(),
        new_text: patch.new_text.clone(),
        reason: patch.reason.clone(),
        risk: patch.risk.clone(),
    }
}
