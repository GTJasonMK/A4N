mod finalization;
mod request;

use a4n_core::error::AppError;

use self::finalization::finalize_planning;
use self::request::generate_planning_trace;
use super::contracts::{MangaExtractionRun, MangaPlanningRun, MangaRunContext};

pub(super) async fn run_planning(
    context: &MangaRunContext<'_>,
    extraction: MangaExtractionRun,
) -> Result<MangaPlanningRun, AppError> {
    let plan_trace = generate_planning_trace(context, &extraction).await?;
    finalize_planning(context, extraction, plan_trace).await
}
