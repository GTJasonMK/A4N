mod adaptation;
mod layout;
mod validation;

#[cfg(test)]
mod tests;

use a4n_core::error::AppError;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    shooting_script::contracts::{
        ChapterScreenplayPlan, PreparedChapterShootingScriptRequest, ShootingScriptProgressReporter,
    },
};

pub(super) async fn generate_screenplay_plan(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    progress_total: u32,
    progress: ShootingScriptProgressReporter<'_>,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChapterScreenplayPlan, AppError> {
    let layout = layout::generate_screenplay_layout(
        orchestrator,
        workflow_run_id,
        prepared,
        progress_total,
        progress,
        cancel_token,
    )
    .await?;
    adaptation::generate_screenplay_adaptations(
        orchestrator,
        workflow_run_id,
        prepared,
        layout,
        progress_total,
        progress,
        cancel_token,
    )
    .await
}
