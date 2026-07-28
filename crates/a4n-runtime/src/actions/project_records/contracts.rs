pub use a4n_api_contract::project::{
    CreateDerivedProjectInput, CreateFreeformVideoProjectInput, CreateProjectInput,
    PaginatedProjects, ProjectGeneratedDataResetSummary, ProjectProgressSummary, ProjectSummary,
    UpdateProjectInput,
};
pub use a4n_orchestrator::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, SourceSnapshotChapterSummary,
};

pub(super) fn contract_progress_summary(
    summary: a4n_orchestrator::project::ProjectProgressSummary,
) -> ProjectProgressSummary {
    ProjectProgressSummary {
        kind: summary.kind,
        stage: summary.stage,
        label: summary.label,
        current: summary.current,
        total: summary.total,
        status: match summary.status {
            a4n_orchestrator::project::ProjectProgressStatus::Idle => "idle",
            a4n_orchestrator::project::ProjectProgressStatus::Running => "running",
            a4n_orchestrator::project::ProjectProgressStatus::Blocked => "blocked",
            a4n_orchestrator::project::ProjectProgressStatus::Failed => "failed",
            a4n_orchestrator::project::ProjectProgressStatus::Completed => "completed",
        }
        .to_string(),
        detail: summary.detail,
    }
}
