mod contracts;
mod drawing;
mod finalization;
mod persistence;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;

use self::drawing::run_completion_drawing;
use self::finalization::finalize_completed_pipeline;
use self::persistence::persist_completion_outputs;
use super::contracts::{MangaPlanningRun, MangaRunContext};

pub(super) async fn complete_pipeline(
    context: &MangaRunContext<'_>,
    planning: MangaPlanningRun,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    let artifacts = run_completion_drawing(context, planning).await?;
    let results = persist_completion_outputs(context, artifacts).await?;
    finalize_completed_pipeline(context, results.len()).await?;
    Ok(results)
}
