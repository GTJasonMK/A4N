pub use a4n_api_contract::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, CreateDerivedProjectInput,
    CreateFreeformVideoProjectInput, CreateProjectInput, PaginatedProjects,
    ProjectGeneratedDataResetSummary, ProjectProgressSummary, ProjectSummary, UpdateProjectInput,
};
pub use a4n_orchestrator::project::SourceSnapshotChapterSummary;

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
            a4n_orchestrator::project::ProjectProgressStatus::Idle => {
                a4n_api_contract::project::ProjectProgressStatus::Idle
            }
            a4n_orchestrator::project::ProjectProgressStatus::Running => {
                a4n_api_contract::project::ProjectProgressStatus::Running
            }
            a4n_orchestrator::project::ProjectProgressStatus::Blocked => {
                a4n_api_contract::project::ProjectProgressStatus::Blocked
            }
            a4n_orchestrator::project::ProjectProgressStatus::Failed => {
                a4n_api_contract::project::ProjectProgressStatus::Failed
            }
            a4n_orchestrator::project::ProjectProgressStatus::Completed => {
                a4n_api_contract::project::ProjectProgressStatus::Completed
            }
        },
        detail: summary.detail,
    }
}
